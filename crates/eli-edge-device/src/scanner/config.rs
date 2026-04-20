use serde::{Deserialize, Serialize};
use crate::scanner::vanilla::IqCaptureMode;
use super::dwell_capture::SettleStrategy;
use super::sweep_planner::{SweepCoverage, SweepExecution, SweepPolicy};


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