use serde::{Deserialize, Serialize};
use crate::edge_vanilla::scanner::dwell_vanilla::SettleStrategy;
use crate::edge_vanilla::scanner::sweep_vanilla::{SweepCoverage, SweepExecution, SweepPolicy};
use crate::edge_vanilla::scanner::msg_vanilla::IqCaptureMode;

pub const DEFAULT_SAMPLE_TIMEOUT : i64 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScannerMode {
    Sweep(SweepModeConfig),
    Fixed(FixedModeConfig),
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepModeConfig {
    pub coverage: SweepCoverage,
    pub execution: SweepExecution,
    pub policy: SweepPolicy,
    pub hotspots: Vec<HotspotConfig>,
    pub fft_min_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedModeConfig {
    pub center_hz: f64,
    pub sample_rate_hz: f64,
    pub dwell_ms: u64,
    pub fft_min_samples: usize,
    pub settle: SettleStrategy,
    pub iq_capture: IqCaptureMode,
    pub iq_chunk_samples: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub source_id: String,
    pub edge_id: String,
    pub sample_rate_hz: f64,
    pub settle: SettleStrategy,
    pub mode: ScannerMode,
}

impl ScannerConfig {
    pub fn default_center_hz(&self) -> f64 {
        match &self.mode {
            ScannerMode::Fixed(cfg) => cfg.center_hz,
            ScannerMode::Sweep(cfg) => cfg.coverage.start_hz,
            ScannerMode::Idle => 96_300_000.0,
        }
    }

    pub fn default_for_worker(worker_id: String) -> Self {
        Self {
            edge_id: worker_id,
            source_id: "rtl-sdr-0".to_string(),
            sample_rate_hz: 2_048_000.0,
            settle: SettleStrategy::SleepAndFlush {
                millis: 5,
                flush_count: 2,
                timeout_us: 250_000,
            },
            mode: ScannerMode::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotConfig {
    pub center_hz: f64,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScannerCommand {
    Start,
    Stop,
    SetConfig(ScannerConfig),
}

#[derive(Debug, Clone)]
pub struct HitDetectorConfig {
    pub min_snr_db: f32,
    pub min_peak_power: f32,
    pub edge_exclusion_bins: usize,
}

impl Default for HitDetectorConfig {
    fn default() -> Self {
        Self {
            min_snr_db: 12.0,
            min_peak_power: 0.0001,
            edge_exclusion_bins: 8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub source_id: String,
    pub center_hz: f64,
    pub peak_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub peak_bin: usize,
    pub peak_power: f32,
    pub noise_floor: f32,
    pub avg_power: f32,
    pub snr_db: f32,
    pub timestamp_ms: u64,
}