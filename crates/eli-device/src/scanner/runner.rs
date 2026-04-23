use std::ops::DerefMut;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use eli_protocol::edge_vanilla::scanner::config_vanilla::{FixedModeConfig, Hit, HitDetectorConfig, ScannerConfig, ScannerMode, SweepModeConfig};
use eli_protocol::edge_vanilla::scanner::dwell_vanilla::SettleStrategy;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::{AnalysisResult, BinValueKind, EdgeEvent, FreqRange, IqCaptureMode, IqChunkMessage, MessageKind, PowerCtx, RecordCtx, RecordMessage, RecordMessageKind, SpectrumFrame, StatusMessage, WaterfallMessage};
use eli_protocol::edge_vanilla::scanner::sweep_vanilla::SweepPolicy;


use crate::scanner::dwell_capture::{dwell_capture};
use crate::scanner::fft_analysis::analyze;
use crate::scanner::hit_detection::detect_hit;
use crate::scanner::sweep_planner::SweepPlanner;


use crate::edge_error::EdgeError;
use crate::{EdgeResult, HOTSPOT_REPRIORITIZE_RADIUS_HZ, HOTSPOT_REPRIORITIZE_WEIGHT, HZ_PER_MHZ, POWER_EPSILON, SCANNER_SLEEP_TIME_MS};
use crate::helpers::dc_dcb::power_to_db;
use crate::scanner::stream_device::stream_vanilla::{DeviceStream};

pub struct ScannerRunner {
    pub stream: Box<dyn DeviceStream>,
    pub active_config: ScannerConfig,
    pub pending_config: Arc<Mutex<Option<ScannerConfig>>>,
    pub scanner_running: Arc<AtomicBool>,
    pub shutdown_requested: Arc<AtomicBool>,
    pub dropped_events: Arc<AtomicU64>,
}

struct EmitContext<'a> {
    edge_id: &'a str,
    source_id: &'a str,
    sample_rate_hz: f64,
    fft_size: usize,
    edge_tx: &'a mpsc::Sender<EdgeEvent>,
    hit_cfg: &'a HitDetectorConfig,
}

impl ScannerRunner {
    pub fn new(
        stream: Box<dyn DeviceStream>,
        config: ScannerConfig,
        pending_config: Arc<Mutex<Option<ScannerConfig>>>,
        scanner_running: Arc<AtomicBool>,
        shutdown_requested: Arc<AtomicBool>,
        dropped_events: Arc<AtomicU64>,
    ) -> Self {
        Self {
            stream,
            active_config: config,
            pending_config,
            scanner_running,
            shutdown_requested,
            dropped_events,
        }
    }

    fn apply_pending_config(&mut self, edge_tx: &mpsc::Sender<EdgeEvent>) -> EdgeResult<bool> {
        let new_cfg = {
            let mut pending = self.pending_config.lock().unwrap();
            pending.take()
        };

        if let Some(new_cfg) = new_cfg {
            eprintln!("[scanner] applying pending config: {:?}", new_cfg.mode);

            self.active_config = new_cfg;

            self.try_emit(
                edge_tx,
                EdgeEvent::Status(StatusMessage::new(
                    self.active_config.edge_id.clone(),
                    self.active_config.source_id.clone(),
                    "config_applied",
                    &format!("active mode now {:?}", self.active_config.mode),
                )),
            );

            return Ok(true);
        }

        Ok(false)
    }

    fn try_emit(&self, edge_tx: &mpsc::Sender<EdgeEvent>, event: EdgeEvent) {
        if edge_tx.try_send(event).is_err() {
            let dropped = self.dropped_events.fetch_add(1, Ordering::Relaxed) + 1;
            if dropped.is_multiple_of(100) {
                eprintln!(
                    "edge event backpressure: dropped_events={} edge_id={} source_id={}",
                    dropped, self.active_config.edge_id, self.active_config.source_id
                );
            }
        }
    }

    fn linear_to_db_bins(bins: &[f32]) -> Vec<f32> {
        bins.iter()
            .map(|v| 10.0 * f32::log10(v.max(POWER_EPSILON)))
            .collect()
    }

    fn build_planner(&self, mode_cfg: &SweepModeConfig) -> SweepPlanner {
        match mode_cfg.policy {
            SweepPolicy::Sequential => SweepPlanner::new_linear(&mode_cfg.coverage),
            SweepPolicy::PriorityHotspots => {
                let hotspot_pairs: Vec<(f64, f32)> = mode_cfg
                    .hotspots
                    .iter()
                    .map(|h| (h.center_hz, h.weight))
                    .collect();

                SweepPlanner::new_priority(&mode_cfg.coverage, &hotspot_pairs)
            }
            SweepPolicy::Randomized => SweepPlanner::new_randomized(&mode_cfg.coverage),
            SweepPolicy::WeightedHotspots => {
                SweepPlanner::new_weighted(&mode_cfg.coverage, &mode_cfg.hotspots)
            }
        }
    }

    fn emit_messages(
        &self,
        analysis: &AnalysisResult,
        timestamp_ms: u128,
        ctx: &EmitContext,
    ) -> EdgeResult<Option<Hit>> {
        let edge_id = ctx.edge_id.to_string();
        let source_id = ctx.source_id.to_string();

        let snr_db = power_to_db(
            (analysis.peak_power / analysis.noise_floor.max(POWER_EPSILON))
                .max(POWER_EPSILON),
        );

        let record_ctx = RecordCtx::new(
            MessageKind::Record,
            edge_id.clone(),
            source_id.clone(),
            timestamp_ms,
        );

        let freq_range = FreqRange::new(
            analysis.lower_edge_hz,
            analysis.upper_edge_hz,
            analysis.center_hz,
        );

        let linear_power_ctx = PowerCtx::new(
            analysis.peak_bin,
            analysis.peak_power,
            analysis.center_hz,
            Some(analysis.estimated_peak_hz),
            analysis.noise_floor,
            analysis.avg_power,
            Some(snr_db),
        );

        let record_msg = RecordMessage {
            record_ctx: record_ctx.clone(),
            freq_range: freq_range.clone(),
            power_ctx: linear_power_ctx.clone(),
            record_message_kind: RecordMessageKind::General,
        };

        self.try_emit(ctx.edge_tx, EdgeEvent::Record(record_msg));

        let spectrum_ctx = RecordCtx::new(
            MessageKind::Spectrum,
            edge_id.clone(),
            source_id.clone(),
            timestamp_ms,
        );

        let linear_frame = SpectrumFrame::new(
            spectrum_ctx.clone(),
            freq_range.clone(),
            ctx.sample_rate_hz,
            ctx.fft_size,
            BinValueKind::LinearPower,
            linear_power_ctx,
            analysis.spectrum.clone(),
        )
        .map_err(|e| format!("failed to build linear spectrum frame: {e}"))?;

        let db_bins = Self::linear_to_db_bins(&analysis.spectrum);
        let db_power_ctx = PowerCtx::new(
            analysis.peak_bin,
            power_to_db(analysis.peak_power.max(POWER_EPSILON)),
            analysis.center_hz,
            Some(analysis.estimated_peak_hz),
            power_to_db(analysis.noise_floor),
            power_to_db(analysis.avg_power),
            Some(snr_db),
        );

        let decibel_frame = SpectrumFrame::new(
            spectrum_ctx,
            freq_range,
            ctx.sample_rate_hz,
            ctx.fft_size,
            BinValueKind::DecibelPower,
            db_power_ctx,
            db_bins,
        )
        .map_err(|e| format!("failed to build dB spectrum frame: {e}"))?;

        let waterfall_ctx = RecordCtx::new(
            MessageKind::Waterfall,
            edge_id.clone(),
            source_id.clone(),
            timestamp_ms,
        );

        let waterfall_msg = WaterfallMessage {
            record_ctx: waterfall_ctx,
            linear: linear_frame,
            decibel: decibel_frame,
        };

        self.try_emit(ctx.edge_tx, EdgeEvent::Waterfall(Box::new(waterfall_msg)));

        if let Some(hit) = detect_hit(
            ctx.hit_cfg,
            ctx.source_id,
            timestamp_ms as u64,
            analysis,
            ctx.fft_size,
        ) {
            log_hit(&hit);

            let hit_record_ctx = RecordCtx::new(
                MessageKind::Record,
                edge_id,
                source_id,
                timestamp_ms,
            );

            let hit_msg = RecordMessage {
                record_ctx: hit_record_ctx,
                freq_range: FreqRange::new(
                    analysis.lower_edge_hz,
                    analysis.upper_edge_hz,
                    analysis.center_hz,
                ),
                power_ctx: PowerCtx::new(
                    analysis.peak_bin,
                    analysis.peak_power,
                    analysis.center_hz,
                    Some(analysis.estimated_peak_hz),
                    analysis.noise_floor,
                    analysis.avg_power,
                    Some(snr_db),
                ),
                record_message_kind: RecordMessageKind::Hit,
            };

            self.try_emit(ctx.edge_tx, EdgeEvent::Record(hit_msg));
            return Ok(Some(hit));
        }

        Ok(None)
    }

    fn run_sweep_mode(
        &mut self,
        mode_cfg: SweepModeConfig,
        edge_tx: &mpsc::Sender<EdgeEvent>,
        hit_cfg: &HitDetectorConfig,
    ) -> EdgeResult<()> {
        let mut planner = self.build_planner(&mode_cfg);
        let edge_id = self.active_config.edge_id.clone();
        let source_id = self.active_config.source_id.clone();
        let sample_rate_hz = self.active_config.sample_rate_hz;
        let settle = self.active_config.settle.clone();

        self.stream.set_sample_rate(self.active_config.sample_rate_hz)?;

        eprintln!(
            "[scanner] configured sample rate now {}",
            self.stream.current_sample_rate()?
        );

        while let Some(point) = planner.pop_next() {
            eprintln!("[sweep] point center_hz={}", point.center_hz);
            if self.shutdown_requested.load(Ordering::Relaxed) {
                break;
            }

            if !self.scanner_running.load(Ordering::SeqCst) {
                break;
            }

            if self.apply_pending_config(edge_tx)? {
                break;
            }

            let samples = match dwell_capture(
                self.stream.as_mut(),
                point.center_hz,
                mode_cfg.execution.dwell_ms,
                &settle,
            ) {
                Ok(samples) => {
                    eprintln!(
                        "[sweep] capture_ok center_hz={} samples={}",
                        point.center_hz,
                        samples.len()
                    );
                    samples
                }
                Err(err) => {
                    eprintln!(
                        "[sweep] capture_err center_hz={} err={}",
                        point.center_hz,
                        err
                    );

                    match self.handle_capture_error(err, point.center_hz)? {
                        Some(samples) => samples,
                        None => continue,
                    }
                }
            };


            if samples.len() < mode_cfg.fft_min_samples {
                continue;
            }

            let analysis = analyze(
                &samples[..mode_cfg.fft_min_samples],
                point.center_hz,
                sample_rate_hz,
            );



            let timestamp_ms = now_ms();
            let emit_context = EmitContext {
                edge_id: &edge_id,
                source_id: &source_id,
                edge_tx,
                hit_cfg,
                sample_rate_hz,
                fft_size: mode_cfg.fft_min_samples,
            };

            let maybe_hit = self.emit_messages(&analysis, timestamp_ms, &emit_context)?;


            if let Some(hit) = maybe_hit.filter(|_| matches!(mode_cfg.policy, SweepPolicy::PriorityHotspots)) {
                planner.reprioritize_near(hit.peak_hz, HOTSPOT_REPRIORITIZE_WEIGHT, HOTSPOT_REPRIORITIZE_RADIUS_HZ);
            }
        }

        Ok(())
    }

    fn run_fixed_mode(
        &mut self,
        mode_cfg: FixedModeConfig,
        edge_tx: &mpsc::Sender<EdgeEvent>,
        hit_cfg: &HitDetectorConfig,
    ) -> EdgeResult<()> {
        let edge_id = self.active_config.edge_id.clone();
        let source_id = self.active_config.source_id.clone();

        loop {
            if self.shutdown_requested.load(Ordering::Relaxed) {
                break;
            }

            if !self.scanner_running.load(Ordering::SeqCst) {
                break;
            }

            if self.apply_pending_config(edge_tx)? {
                break;
            }

            let samples = match dwell_capture(
                self.stream.as_mut(),
                mode_cfg.center_hz,
                mode_cfg.dwell_ms,
                &mode_cfg.settle,
            ) {
                Ok(samples) => samples,
                Err(err) => match self.handle_capture_error(err, mode_cfg.center_hz)? {
                    Some(samples) => samples,
                    None => continue,
                },
            };

            if samples.len() < mode_cfg.fft_min_samples {
                continue;
            }

            let analysis = analyze(
                &samples[..mode_cfg.fft_min_samples],
                mode_cfg.center_hz,
                mode_cfg.sample_rate_hz,
            );

            let timestamp_ms = now_ms();

            if matches!(mode_cfg.iq_capture, IqCaptureMode::Stream) {
                let record_ctx = RecordCtx::new(
                    MessageKind::Iq,
                    edge_id.clone(),
                    source_id.clone(),
                    timestamp_ms,
                );

                let bounded_len = mode_cfg.iq_chunk_samples.min(samples.len());
                let iq_msg = IqChunkMessage::new(
                    record_ctx,
                    mode_cfg.center_hz,
                    mode_cfg.sample_rate_hz,
                    &samples[..bounded_len],
                );

                self.try_emit(edge_tx, EdgeEvent::IqChunk(iq_msg));
            }

            let emit_ctx = EmitContext {
                edge_id: &edge_id,
                source_id: &source_id,
                edge_tx,
                hit_cfg,
                sample_rate_hz: mode_cfg.sample_rate_hz,
                fft_size: mode_cfg.fft_min_samples,
            };

            let _ = self.emit_messages(&analysis, timestamp_ms, &emit_ctx)?;
        }

        Ok(())
    }

    pub fn run_edge_loop(mut self, edge_tx: mpsc::Sender<EdgeEvent>) -> EdgeResult<()> {


        let hit_cfg = HitDetectorConfig::default();



        self.stream.activate()?;



        loop {
            let running = self.scanner_running.load(Ordering::SeqCst);
            let mode = format!("{:?}", self.active_config.mode);


            if self.shutdown_requested.load(Ordering::SeqCst) {
                break;
            }

            if self.apply_pending_config(&edge_tx)? {
                continue;
            }

            if !self.scanner_running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(SCANNER_SLEEP_TIME_MS));
                continue;
            }

            match self.active_config.mode.clone() {
                ScannerMode::Sweep(mode_cfg) => {

                    self.try_emit(
                        &edge_tx,
                        EdgeEvent::Status(StatusMessage::new(
                            self.active_config.edge_id.clone(),
                            self.active_config.source_id.clone(),
                            "mode_enter",
                            "entering sweep mode",
                        )),
                    );

                    self.run_sweep_mode(mode_cfg, &edge_tx, &hit_cfg)?;
                }
                ScannerMode::Fixed(mode_cfg) => {
                    self.try_emit(
                        &edge_tx,
                        EdgeEvent::Status(StatusMessage::new(
                            self.active_config.edge_id.clone(),
                            self.active_config.source_id.clone(),
                            "mode_enter",
                            "entering fixed mode",
                        )),
                    );

                    self.run_fixed_mode(mode_cfg, &edge_tx, &hit_cfg)?;
                }
                ScannerMode::Idle => {
                    self.try_emit(
                        &edge_tx,
                        EdgeEvent::Status(StatusMessage::new(
                            self.active_config.edge_id.clone(),
                            self.active_config.source_id.clone(),
                            "mode_enter",
                            "idle mode",
                        )),
                    );

                    std::thread::sleep(Duration::from_millis(SCANNER_SLEEP_TIME_MS));
                }
            }

            eprintln!(
                "[scanner] tick running={} mode={:?}",
                self.scanner_running.load(Ordering::SeqCst),
                self.active_config.mode
            );
        }

        Ok(())
    }

    fn handle_capture_error<T>(
        &self,
        err: EdgeError,
        center_hz: f64,
    ) -> EdgeResult<Option<T>> {
        if is_overflow_error(&err) {
            eprintln!(
                "scanner overflow at {:.3} MHz; continuing",
                center_hz / HZ_PER_MHZ
            );
            return Ok(None);
        }

        Err(err)
    }
}

fn log_hit(hit: &Hit) {
    println!("Hit detected: {:?}", hit);
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn is_overflow_error(err: &EdgeError) -> bool {
    err.to_string().contains("Overflow")
}
