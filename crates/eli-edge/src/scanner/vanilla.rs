use serde::{Deserialize, Serialize};

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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMessage {
    pub r#type: String, // "record"

    pub edge_id: String,
    pub source_id: String,
    pub timestamp_ms: u128,

    pub center_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,

    pub peak_bin: usize,
    pub peak_hz: f64,
    pub peak_power: f32,

    pub noise_floor: f32,
    pub avg_power: f32,
    pub snr_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitMessage {
    pub r#type: String, // "hit"

    pub edge_id: String,
    pub source_id: String,
    pub timestamp_ms: u128,

    pub center_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,

    pub peak_bin: usize,
    pub peak_hz: f64,
    pub peak_power: f32,

    pub noise_floor: f32,
    pub avg_power: f32,
    pub snr_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallMessage {
    pub r#type: String, // "waterfall_frame"
    pub timestamp_ms: u128,
    pub edge_id: String,
    pub source_id: String,

    pub linear: SpectrumFrame,
    pub decibel: SpectrumFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Bin {
    index: usize,
    hz: f64,
    power: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinValueKind {
    LinearPower,
    DecibelPower,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumFrame {
    pub timestamp_ms: u128,
    pub edge_id: String,
    pub source_id: String,

    pub center_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub bin_width_hz: f64,

    pub sample_rate_hz: f64,
    pub fft_size: usize,

    pub value_kind: BinValueKind,

    pub peak_bin: usize,
    pub peak_hz: f64,
    pub peak_power: f32,
    pub noise_floor: f32,
    pub avg_power: f32,

    pub bins: Vec<f32>,
}


impl SpectrumFrame {
    pub fn new(
        timestamp_ms: u128,
        edge_id: String,
        source_id: String,
        center_hz: f64,
        lower_edge_hz: f64,
        upper_edge_hz: f64,
        sample_rate_hz: f64,
        fft_size: usize,
        value_kind: BinValueKind,
        peak_bin: usize,
        peak_power: f32,
        noise_floor: f32,
        avg_power: f32,
        bins: Vec<f32>,
    ) -> Result<Self, String> {
        if bins.len() != fft_size {
            return Err(format!("bins len {} != fft_size {}", bins.len(), fft_size));
        }

        if fft_size == 0 {
            return Err("fft_size must be > 0".to_string());
        }

        if !(upper_edge_hz > lower_edge_hz) {
            return Err("upper_edge_hz must be > lower_edge_hz".to_string());
        }

        if peak_bin >= bins.len() {
            return Err(format!("peak_bin {} out of range {}", peak_bin, bins.len()));
        }

        let bin_width_hz = (upper_edge_hz - lower_edge_hz) / fft_size as f64;
        let peak_hz = lower_edge_hz + (peak_bin as f64 * bin_width_hz);

        Ok(Self {
            timestamp_ms,
            edge_id,
            source_id,
            center_hz,
            lower_edge_hz,
            upper_edge_hz,
            bin_width_hz,
            sample_rate_hz,
            fft_size,
            value_kind,
            peak_bin,
            peak_hz,
            peak_power,
            noise_floor,
            avg_power,
            bins,
        })
    }

    pub fn hz_for_bin(&self, bin: usize) -> f64 {
        self.lower_edge_hz + (bin as f64 * self.bin_width_hz)
    }
}
