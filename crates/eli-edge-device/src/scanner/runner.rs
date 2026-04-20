use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::scanner::config::*;
use crate::scanner::dwell_capture::dwell_capture;
use crate::scanner::fft_analysis::{analyze, AnalysisResult};
use crate::scanner::hit_detection::{detect_hit, Hit, HitDetectorConfig};
use crate::scanner::sweep_planner::{SweepPlanner, SweepPolicy};
use crate::scanner::vanilla::{BinValueKind, EdgeEvent, FreqRange, IqCaptureMode, IqChunkMessage, MessageKind, PowerCtx, RecordCtx, RecordMessage, RecordMessageKind, SpectrumFrame, WaterfallMessage};
use tokio::sync::mpsc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ScannerRunner {
    pub stream: crate::capture::stream::RtlStream,
    pub active_config: ScannerConfig,
    pub pending_config: Option<ScannerConfig>,
    pub scanner_running: Arc<AtomicBool>,
}

struct EmitContext<'a> {
    edge_id: &'a str,
    source_id: &'a str,
    sample_rate_hz: f64,
    fft_size: usize,
    edge_tx : &'a mpsc::Sender<EdgeEvent>,
    hit_cfg: &'a HitDetectorConfig,
}

impl ScannerRunner {
    pub fn new(
        stream: crate::capture::stream::RtlStream,
        config: ScannerConfig,
        scanner_running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            stream,
            active_config: config,
            pending_config: None,
            scanner_running,
        }
    }

    fn apply_pending_config(&mut self) -> Result<bool> {
        if let Some(new_cfg) = self.pending_config.take() {
            self.active_config = new_cfg;
            return Ok(true);
        }

        Ok(false)
    }

    fn linear_to_db_bins(bins: &[f32]) -> Vec<f32> {
        bins.iter()
            .map(|v| 10.0 * f32::log10(v.max(1e-12)))
            .collect()
    }

    fn build_planner(&self, mode_cfg: &SweepModeConfig) -> SweepPlanner {
        match mode_cfg.policy {
            SweepPolicy::Sequential => {
                SweepPlanner::new_linear(&mode_cfg.coverage)
            }
            SweepPolicy::PriorityHotspots => {
                let hotspot_pairs: Vec<(f64, f32)> = mode_cfg
                    .hotspots
                    .iter()
                    .map(|h| (h.center_hz, h.weight))
                    .collect();

                SweepPlanner::new_priority(&mode_cfg.coverage, &hotspot_pairs)
            }
            SweepPolicy::Randomized => {
                SweepPlanner::new_randomized(&mode_cfg.coverage)
            }
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
    ) -> Result<Option<Hit>> {
        let edge_id = ctx.edge_id.to_string();
        let source_id = ctx.source_id.to_string();



        let snr_db = 10.0 * f32::log10(
            (analysis.peak_power / analysis.noise_floor.max(1e-12)).max(1e-12)
        );

        let record_ctx = RecordCtx {
            r#type: MessageKind::Record.as_str().to_string(),
            edge_id: edge_id.clone(),
            source_id: source_id.clone(),
            timestamp_ms,
        };

        let freq_range = FreqRange{
            center_hz: analysis.center_hz,
            lower_edge_hz: analysis.lower_edge_hz,
            upper_edge_hz: analysis.upper_edge_hz,
        };

        let power_ctx = PowerCtx::new(analysis.peak_bin, analysis.peak_power, analysis.center_hz, Some(analysis.estimated_peak_hz), analysis.noise_floor, analysis.avg_power, Some(snr_db));

        let record_msg = RecordMessage {
            record_ctx:record_ctx.clone(),
            freq_range,
            power_ctx,
            record_message_kind : RecordMessageKind::General,
        };

        let edge_event = EdgeEvent::Record(record_msg);

        let _ = ctx.edge_tx.try_send(edge_event).ok();


        let freq_range = FreqRange{
            center_hz: analysis.center_hz,
            lower_edge_hz: analysis.lower_edge_hz,
            upper_edge_hz: analysis.upper_edge_hz,
        };

        let power_ctx = PowerCtx::new(analysis.peak_bin, analysis.peak_power, analysis.center_hz, Some(analysis.estimated_peak_hz), analysis.noise_floor, analysis.avg_power, Some(snr_db));


        let record_ctx = RecordCtx {
            r#type: MessageKind::Spectrum.as_str().to_string(),
            edge_id: edge_id.clone(),
            source_id: source_id.clone(),
            timestamp_ms,
        };

        let linear_frame = SpectrumFrame::new(
            record_ctx.clone(),
            freq_range.clone(),
            ctx.sample_rate_hz,
            ctx.fft_size,
            BinValueKind::LinearPower,
            power_ctx,
            analysis.spectrum.clone(),
        )
            .map_err(|e| format!("failed to build linear spectrum frame: {e}"))?;

        let db_bins = Self::linear_to_db_bins(&analysis.spectrum);

        let db_peak_power = 10.0 * f32::log10(analysis.peak_power.max(1e-12));
        let db_noise_floor = 10.0 * f32::log10(analysis.noise_floor.max(1e-12));
        let db_avg_power = 10.0 * f32::log10(analysis.avg_power.max(1e-12));

        let power_ctx = PowerCtx::new(analysis.peak_bin, db_peak_power,
                                      analysis.center_hz, Some(analysis.estimated_peak_hz),
                                      db_noise_floor, db_avg_power, Some(snr_db));

        let decibel_frame = SpectrumFrame::new(
            record_ctx,
            freq_range,
            ctx.sample_rate_hz,
            ctx.fft_size,
            BinValueKind::DecibelPower,
            power_ctx,
            db_bins,
        )
            .map_err(|e| format!("failed to build dB spectrum frame: {e}"))?;


        let record_ctx = RecordCtx {
            r#type: MessageKind::Waterfall.as_str().to_string(),
            timestamp_ms,
            edge_id: edge_id.clone(),
            source_id: source_id.clone(),
        };

        let waterfall_msg = WaterfallMessage {
            record_ctx,
            linear: linear_frame,
            decibel: decibel_frame,
        };

        let edge_event = EdgeEvent::Waterfall(Box::new(waterfall_msg));

        let _ = ctx.edge_tx.try_send(edge_event).ok();

        if let Some(hit) = detect_hit(
            ctx.hit_cfg,
            ctx.source_id,
            timestamp_ms as u64,
            analysis,
            ctx.fft_size,
        ) {
            log_hit(&hit);

            let record_ctx = RecordCtx {
                r#type: MessageKind::Record.as_str().to_string(),
                edge_id: edge_id.clone(),
                source_id: source_id.clone(),
                timestamp_ms,
            };

            let freq_range = FreqRange{
                center_hz: analysis.center_hz,
                lower_edge_hz: analysis.lower_edge_hz,
                upper_edge_hz: analysis.upper_edge_hz,
            };

            let power_ctx = PowerCtx::new(analysis.peak_bin,
                                          analysis.peak_power,
                                          analysis.center_hz,
                                          Some(analysis.estimated_peak_hz),
                                          analysis.noise_floor,
                                          analysis.avg_power,
                                          Some(snr_db));

            let record_msg = RecordMessage {
                record_ctx,
                freq_range,
                power_ctx,
                record_message_kind : RecordMessageKind::Hit,
            };

            let edge_event = EdgeEvent::Record(record_msg);

            let _ = ctx.edge_tx.try_send(edge_event).ok();

            return Ok(Some(hit));
        }

        Ok(None)
    }

    fn run_sweep_mode(
        &mut self,
        mode_cfg: SweepModeConfig,
        edge_tx: &mpsc::Sender<EdgeEvent>,
        hit_cfg: &HitDetectorConfig,
    ) -> Result<()> {
        let mut planner = self.build_planner(&mode_cfg);
        let edge_id = self.active_config.edge_id.clone();
        let source_id = self.active_config.source_id.clone();
        let sample_rate_hz = self.active_config.sample_rate_hz;
        let settle = self.active_config.settle.clone();

        while let Some(point) = planner.pop_next() {
            if !self.scanner_running.load(Ordering::Relaxed) {
                break;
            }

            if self.apply_pending_config()? {
                break;
            }

            let samples = match dwell_capture(
                &mut self.stream,
                point.center_hz,
                mode_cfg.execution.dwell_ms,
                &settle,
            ) {
                Ok(samples) => samples,
                Err(err) => match self.handle_capture_error(err, point.center_hz)? {
                    Some(samples) => samples, // (won’t happen here, but keeps pattern clean)
                    None => continue,
                },
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

            if let Some(hit) = maybe_hit
                .filter(|_| matches!(mode_cfg.policy, SweepPolicy::PriorityHotspots))
            {
                planner.reprioritize_near(hit.peak_hz, 0.75, 1_500_000.0);
            }
        }

        Ok(())
    }

    fn run_fixed_mode(
        &mut self,
        mode_cfg: FixedModeConfig,
        edge_tx: &mpsc::Sender<EdgeEvent>,
        hit_cfg: &HitDetectorConfig,
    ) -> Result<()> {
        let edge_id = self.active_config.edge_id.clone();
        let source_id = self.active_config.source_id.clone();

        loop {
            if !self.scanner_running.load(Ordering::Relaxed) {
                break;
            }

            if self.apply_pending_config()? {
                break;
            }

            let samples = match dwell_capture(
                &mut self.stream,
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

                let record_ctx = RecordCtx {
                    r#type: MessageKind::Iq.as_str().to_string(),
                    edge_id: edge_id.clone(),
                    source_id: source_id.clone(),
                    timestamp_ms,
                };

                let iq_msg = IqChunkMessage::new(
                    record_ctx,
                    mode_cfg.center_hz,
                    mode_cfg.sample_rate_hz,
                    &samples[..mode_cfg.iq_chunk_samples.min(samples.len())],
                );

                let edge_event = EdgeEvent::IqChunk(iq_msg);
                let _ = edge_tx.try_send(edge_event).ok();

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

    pub fn run_edge_loop(
        &mut self,
        edge_tx: mpsc::Sender<EdgeEvent>,
    ) -> Result<()> {
        let hit_cfg = HitDetectorConfig::default();

        self.stream.activate()?;

        loop {
            if !self.scanner_running.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            self.apply_pending_config()?;

            match self.active_config.mode.clone() {
                ScannerMode::Sweep(mode_cfg) => {
                    self.run_sweep_mode(
                        mode_cfg,
                        &edge_tx,
                        &hit_cfg,
                    )?;
                }
                ScannerMode::Fixed(mode_cfg) => {
                    self.run_fixed_mode(
                        mode_cfg,
                        &edge_tx,
                        &hit_cfg,
                    )?;
                }
                ScannerMode::Idle => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
    fn handle_capture_error<T>(
        &self,
        err: Box<dyn std::error::Error>,
        center_hz: f64,
    ) -> Result<Option<T>> {
        if is_overflow_error(err.as_ref()) {
            eprintln!(
                "scanner overflow at {:.3} MHz; continuing",
                center_hz / 1_000_000.0
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

fn is_overflow_error(err: &dyn std::error::Error) -> bool {
    err.to_string().contains("Overflow")
}



