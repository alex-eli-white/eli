use num_complex::Complex32;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    Record,
    Hit,
    Waterfall,
    Iq,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageKind::Record => "record",
            MessageKind::Hit => "hit",
            MessageKind::Waterfall => "waterfall_frame",
            MessageKind::Iq => "iq_chunk",
        }
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakFinder {
    pub peak_bin: usize,
    pub peak_power: f32,

    pub bin_center_hz: f64,
    pub estimated_peak_hz: Option<f64>,
}

impl PeakFinder {
    pub fn new(
        peak_bin: usize,
        peak_power: f32,
        bin_center_hz: f64,
        estimated_peak_hz: Option<f64>,
    ) -> Self {
        Self {
            peak_bin,
            peak_power,
            bin_center_hz,
            estimated_peak_hz,
        }
    }

    pub fn best_hz(&self) -> f64 {
        self.estimated_peak_hz.unwrap_or(self.bin_center_hz)
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterfallMessage {
    pub record_ctx : RecordCtx,

    pub linear: SpectrumFrame,
    pub decibel: SpectrumFrame,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordCtx{
    pub r#type : String,
    pub edge_id : String,
    pub source_id : String,
    pub timestamp_ms : u128,
}

impl RecordCtx{
    pub fn new(r#type : String, edge_id : String, source_id : String, timestamp_ms : u128) -> Self{
        Self{
            r#type,
            edge_id,
            source_id,
            timestamp_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreqRange {
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub center_hz: f64,
}

impl FreqRange {
    pub fn new(lower_edge_hz: f64, upper_edge_hz: f64, center_hz: f64) -> Self {
        Self {
            lower_edge_hz,
            upper_edge_hz,
            center_hz,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinValueKind {
    LinearPower,
    DecibelPower,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMessage {
    pub record_ctx : RecordCtx,

    pub freq_range: FreqRange,

    pub peak : PeakFinder,

    pub record_message_kind: RecordMessageKind,

    pub noise_floor: f32,
    pub avg_power: f32,
    pub snr_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordMessageKind {
    Hit,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumFrame {
    pub record_ctx : RecordCtx,

    pub freq_range: FreqRange,

    pub value_kind: BinValueKind,

    pub peak : PeakFinder,

    pub bin_width_hz: f64,

    pub sample_rate_hz: f64,
    pub fft_size: usize,

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

        let record_ctx = RecordCtx::new("spectrum_frame".to_string(), edge_id, source_id, timestamp_ms);

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
        let peak = PeakFinder::new(peak_bin,peak_power,  peak_hz);
        let freq_range = FreqRange::new(lower_edge_hz, upper_edge_hz, center_hz);

        Ok(Self {
            record_ctx,
            freq_range,
            bin_width_hz,
            sample_rate_hz,
            fft_size,
            value_kind,
            peak,
            noise_floor,
            avg_power,
            bins,
        })
    }

    pub fn hz_for_bin(&self, bin: usize) -> f64 {
        self.freq_range.lower_edge_hz + (bin as f64 * self.bin_width_hz)
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IqChunkMessage {
    // pub r#type: String,
    //
    // pub edge_id: String,
    // pub source_id: String,
    // pub timestamp_ms: u128,

    pub record_ctx: RecordCtx,

    pub center_hz: f64,
    pub sample_rate_hz: f64,

    pub sample_format: IqSampleFormat,
    pub sample_count: usize,

    pub samples_i: Vec<f32>,
    pub samples_q: Vec<f32>,
}

impl IqChunkMessage {
    pub fn new(
        record_ctx: RecordCtx,
        center_hz: f64,
        sample_rate_hz: f64,
        samples: &[Complex32]
    )->Self{
        Self{
            record_ctx,
            center_hz,
            sample_rate_hz,
            sample_format: IqSampleFormat::ComplexF32,
            sample_count: samples.len() as u32 as usize,
            samples_i: samples.iter().map(|s| s.re).collect(),
            samples_q: samples.iter().map(|s| s.im).collect(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMessage {
    pub r#type: String,

    pub edge_id: String,
    pub source_id: String,
    pub timestamp_ms: u128,

    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IqSampleFormat {
    ComplexF32,
}

#[derive(Debug, Clone, Copy)]
pub enum IqCaptureMode {
    Off,
    Stream,
    Snapshot,
}

pub enum EdgeEvent {
    Status(StatusMessage),
    Record(RecordMessage),
    Waterfall(Box<WaterfallMessage>),
    IqChunk(IqChunkMessage),
}