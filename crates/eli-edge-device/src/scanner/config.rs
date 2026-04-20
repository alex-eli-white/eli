use crate::scanner::vanilla::IqCaptureMode;
use super::dwell_capture::SettleStrategy;
use super::sweep_planner::{SweepCoverage, SweepExecution, SweepPolicy};

#[derive(Debug, Clone)]
pub enum ScannerMode {
    Sweep(SweepModeConfig),
    Fixed(FixedModeConfig),
}

#[derive(Debug, Clone)]
pub struct SweepModeConfig {
    pub coverage: SweepCoverage,
    pub execution: SweepExecution,
    pub policy: SweepPolicy,
    pub hotspots: Vec<HotspotConfig>,
    pub fft_min_samples: usize,
}

#[derive(Debug, Clone)]
pub struct FixedModeConfig {
    pub center_hz: f64,
    pub sample_rate_hz: f64,
    pub dwell_ms: u64,
    pub fft_min_samples: usize,
    pub settle: SettleStrategy,
    pub iq_capture: IqCaptureMode,
    pub iq_chunk_samples: usize,
}
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub source_id: String,
    pub edge_id: String,
    pub sample_rate_hz: f64,
    pub settle: SettleStrategy,
    pub mode: ScannerMode,
}

#[derive(Debug, Clone, Copy)]
pub struct HotspotConfig {
    pub center_hz: f64,
    pub weight: f32,
}

pub enum ScannerCommand {
    Start,
    Stop,
    SetConfig(ScannerConfig),
}