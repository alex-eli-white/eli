#[derive(Debug, Clone)]
pub struct ScanHit {
    pub dwell_center_hz: f64,
    pub estimated_peak_hz: f64,
    pub avg_power: f32,
    pub noise_floor: f32,
    pub peak_bin: usize,
    pub peak_power: f32,
    pub snr_like_db: f32,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SweepRecord {
    pub center_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub avg_power: f32,
    pub noise_floor: f32,
    pub peak_power: f32,
    pub peak_bin: usize,
    pub estimated_peak_hz: f64,
    pub timestamp_ms: u64,
}
