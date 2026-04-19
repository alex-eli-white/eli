use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;
use SweepPolicy::PriorityHotspots;
use crate::scanner::config::*;
use crate::scanner::dwell_capture::dwell_capture;
use crate::scanner::fft_analysis::{analyze, AnalysisResult};
use crate::scanner::hit_detection::{detect_hit, Hit, HitDetectorConfig};
use crate::scanner::sweep_planner::{SweepPlanner, SweepPolicy};
use crate::scanner::vanilla::{
    BinValueKind, HitMessage, RecordMessage, SpectrumFrame, WaterfallMessage,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ScannerRunner {
    pub stream: crate::capture::stream::RtlStream,
    pub active_config: ScannerConfig,
    pub pending_config: Option<ScannerConfig>,
    pub scanner_running: Arc<AtomicBool>,
    pub hotspots: Vec<HotspotConfig>,
}

struct EmitContext<'a> {
    source_id: &'a str,
    sample_rate_hz: f64,
    fft_size: usize,
    record_tx: &'a broadcast::Sender<RecordMessage>,
    hit_tx: &'a broadcast::Sender<HitMessage>,
    waterfall_tx: &'a broadcast::Sender<WaterfallMessage>,
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
            hotspots: Vec::new(),
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
                SweepPlanner::new_linear(mode_cfg.coverage)
            }
            PriorityHotspots => {
                let hotspot_pairs: Vec<(f64, f32)> = mode_cfg
                    .hotspots
                    .iter()
                    .map(|h| (h.center_hz, h.weight))
                    .collect();

                SweepPlanner::new_priority(mode_cfg.coverage, &hotspot_pairs)
            }
            SweepPolicy::Randomized => {
                SweepPlanner::new_randomized(mode_cfg.coverage)
            }
            SweepPolicy::WeightedHotspots => {
                SweepPlanner::new_weighted(mode_cfg.coverage, &mode_cfg.hotspots)
            }
        }
    }

    fn emit_messages(
        &self,
        analysis: &AnalysisResult,
        timestamp_ms: u128,
        ctx: &EmitContext,
    ) -> Result<Option<Hit>> {
        let edge_id = self.active_config.edge_id.clone();
        let source_id = self.active_config.source_id.clone();

        let snr_db = 10.0 * f32::log10(
            (analysis.peak_power / analysis.noise_floor.max(1e-12)).max(1e-12)
        );

        let record_msg = RecordMessage {
            r#type: "record".to_string(),
            edge_id: edge_id.clone(),
            source_id: source_id.clone(),
            timestamp_ms,
            center_hz: analysis.center_hz,
            lower_edge_hz: analysis.lower_edge_hz,
            upper_edge_hz: analysis.upper_edge_hz,
            peak_bin: analysis.peak_bin,
            peak_hz: analysis.estimated_peak_hz,
            peak_power: analysis.peak_power,
            noise_floor: analysis.noise_floor,
            avg_power: analysis.avg_power,
            snr_db,
        };

        let _ = ctx.record_tx.send(record_msg);

        let linear_frame = SpectrumFrame::new(
            timestamp_ms,
            edge_id.clone(),
            source_id.clone(),
            analysis.center_hz,
            analysis.lower_edge_hz,
            analysis.upper_edge_hz,
            ctx.sample_rate_hz,
            ctx.fft_size,
            BinValueKind::LinearPower,
            analysis.peak_bin,
            analysis.peak_power,
            analysis.noise_floor,
            analysis.avg_power,
            analysis.spectrum.clone(),
        )
            .map_err(|e| format!("failed to build linear spectrum frame: {e}"))?;

        let db_bins = Self::linear_to_db_bins(&analysis.spectrum);

        let db_peak_power = 10.0 * f32::log10(analysis.peak_power.max(1e-12));
        let db_noise_floor = 10.0 * f32::log10(analysis.noise_floor.max(1e-12));
        let db_avg_power = 10.0 * f32::log10(analysis.avg_power.max(1e-12));

        let decibel_frame = SpectrumFrame::new(
            timestamp_ms,
            edge_id.clone(),
            source_id.clone(),
            analysis.center_hz,
            analysis.lower_edge_hz,
            analysis.upper_edge_hz,
            ctx.sample_rate_hz,
            ctx.fft_size,
            BinValueKind::DecibelPower,
            analysis.peak_bin,
            db_peak_power,
            db_noise_floor,
            db_avg_power,
            db_bins,
        )
            .map_err(|e| format!("failed to build dB spectrum frame: {e}"))?;

        let waterfall_msg = WaterfallMessage {
            r#type: "waterfall_frame".to_string(),
            timestamp_ms,
            edge_id: edge_id.clone(),
            source_id: source_id.clone(),
            linear: linear_frame,
            decibel: decibel_frame,
        };

        let _ = ctx.waterfall_tx.send(waterfall_msg);

        if let Some(hit) = detect_hit(
            ctx.hit_cfg,
            ctx.source_id,
            timestamp_ms as u64,
            analysis,
            ctx.fft_size,
        ) {
            log_hit(&hit);

            let hit_msg = HitMessage {
                r#type: "hit".to_string(),
                edge_id,
                source_id: hit.source_id.clone(),
                timestamp_ms,
                center_hz: hit.center_hz,
                lower_edge_hz: hit.lower_edge_hz,
                upper_edge_hz: hit.upper_edge_hz,
                peak_bin: hit.peak_bin,
                peak_hz: hit.peak_hz,
                peak_power: hit.peak_power,
                noise_floor: hit.noise_floor,
                avg_power: hit.avg_power,
                snr_db: hit.snr_db,
            };

            let _ = ctx.hit_tx.send(hit_msg);
            return Ok(Some(hit));
        }

        Ok(None)
    }

    fn run_sweep_mode(
        &mut self,
        mode_cfg: SweepModeConfig,
        record_tx: &broadcast::Sender<RecordMessage>,
        hit_tx: &broadcast::Sender<HitMessage>,
        waterfall_tx: &broadcast::Sender<WaterfallMessage>,
        hit_cfg: &HitDetectorConfig,
    ) -> Result<()> {
        let mut planner = self.build_planner(&mode_cfg);
        let source_id = self.active_config.source_id.clone();
        let sample_rate_hz = self.active_config.sample_rate_hz;
        let settle = self.active_config.settle;

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
                settle,
            ) {
                Ok(samples) => samples,
                Err(err) => {
                    let msg = err.to_string();

                    if msg.contains("Overflow") {
                        eprintln!(
                            "scanner overflow at {:.3} MHz; continuing",
                            point.center_hz / 1_000_000.0
                        );
                        continue;
                    }

                    return Err(err);
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
                record_tx,
                hit_tx,
                waterfall_tx,
                hit_cfg,
                source_id: &source_id,
                sample_rate_hz,
                fft_size: mode_cfg.fft_min_samples,
            };

            let maybe_hit = self.emit_messages(
                &analysis,
                timestamp_ms,
                &emit_context,
            )?;

            if let Some(hit) = maybe_hit.filter(|_| matches!(mode_cfg.policy, PriorityHotspots)) {
                planner.reprioritize_near(hit.peak_hz, 0.75, 1_500_000.0);
            }
        }

        Ok(())
    }

    fn run_fixed_mode(
        &mut self,
        mode_cfg: FixedModeConfig,
        record_tx: &broadcast::Sender<RecordMessage>,
        hit_tx: &broadcast::Sender<HitMessage>,
        waterfall_tx: &broadcast::Sender<WaterfallMessage>,
        hit_cfg: &HitDetectorConfig,
    ) -> Result<()> {
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
                mode_cfg.settle,
            ) {
                Ok(samples) => samples,
                Err(err) => {
                    let msg = err.to_string();

                    if msg.contains("Overflow") {
                        eprintln!(
                            "fixed mode overflow at {:.3} MHz; continuing",
                            mode_cfg.center_hz / 1_000_000.0
                        );
                        continue;
                    }

                    return Err(err);
                }
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

            let emit_ctx = EmitContext{
                record_tx,
                hit_tx,
                waterfall_tx,
                hit_cfg,
                source_id: &source_id,
                sample_rate_hz: mode_cfg.sample_rate_hz,
                fft_size: mode_cfg.fft_min_samples,
            };

            let _ = self.emit_messages(
                &analysis,
                timestamp_ms,
                &emit_ctx,
            )?;
        }

        Ok(())
    }

    pub fn run_scan_loop(
        &mut self,
        record_tx: broadcast::Sender<RecordMessage>,
        hit_tx: broadcast::Sender<HitMessage>,
        waterfall_tx: broadcast::Sender<WaterfallMessage>,
    ) -> Result<()> {
        let hit_cfg = HitDetectorConfig::default();

        self.stream.activate()?;

        loop {
            if !self.scanner_running.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            let _ = self.apply_pending_config()?;

            match self.active_config.mode.clone() {
                ScannerMode::Sweep(mode_cfg) => {
                    self.run_sweep_mode(
                        mode_cfg,
                        &record_tx,
                        &hit_tx,
                        &waterfall_tx,
                        &hit_cfg,
                    )?;
                }
                ScannerMode::Fixed(mode_cfg) => {
                    self.run_fixed_mode(
                        mode_cfg,
                        &record_tx,
                        &hit_tx,
                        &waterfall_tx,
                        &hit_cfg,
                    )?;
                }
            }
        }
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