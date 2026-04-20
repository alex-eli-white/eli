use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepPoint {
    pub center_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub priority: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepCoverage {
    pub start_hz: f64,
    pub end_hz: f64,
    pub sample_rate_hz: f64,
    pub usable_bandwidth_hz: f64,
    pub overlap_fraction: f64,
}

impl SweepCoverage {
    pub fn step_hz(&self) -> f64 {
        self.usable_bandwidth_hz * (1.0 - self.overlap_fraction)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SweepPolicy {
    Sequential,
    PriorityHotspots,
    WeightedHotspots,
    Randomized,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepExecution {
    pub dwell_ms: u64,
    pub settle_ms: u64,
    pub flush_count: u32,
}
