use num_complex::Complex32;
use serde::{Deserialize, Serialize};
use crate::edge_vanilla::scanner::config_vanilla::ScannerConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageKind {
    Record,
    Hit,
    Waterfall,
    Iq,
    Spectrum,
    Status,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageKind::Record => "record",
            MessageKind::Hit => "hit",
            MessageKind::Waterfall => "waterfall_frame",
            MessageKind::Iq => "iq_chunk",
            MessageKind::Spectrum => "spectrum_frame",
            MessageKind::Status => "status",

        }
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerCtx {
    pub peak_bin: usize,
    pub peak_power: f32,

    pub bin_center_hz: f64,
    pub estimated_peak_hz: Option<f64>,

    pub noise_floor: f32,
    pub avg_power: f32,
    pub snr_db: Option<f32>,
}

impl PowerCtx {
    pub fn new(
        peak_bin: usize,
        peak_power: f32,
        bin_center_hz: f64,
        estimated_peak_hz: Option<f64>,
        noise_floor: f32,
        avg_power: f32,
        snr_db: Option<f32>,
    ) -> Self {
        Self {
            peak_bin,
            peak_power,
            bin_center_hz,
            estimated_peak_hz,
            noise_floor,
            avg_power,
            snr_db,
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

impl RecordCtx {
    pub fn new(kind: MessageKind, edge_id: String, source_id: String, timestamp_ms: u128) -> Self {
        Self {
            r#type: kind.as_str().to_string(),
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
pub enum EdgeCommand {
    SetConfig(ScannerConfig),
    Start,
    Stop,
    Ping,
    Shutdown,
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

    pub power_ctx: PowerCtx,

    pub record_message_kind: RecordMessageKind,

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

    pub power_ctx: PowerCtx,

    pub bin_width_hz: f64,

    pub sample_rate_hz: f64,
    pub fft_size: usize,

    pub bins: Vec<f32>,
}


impl SpectrumFrame {
    pub fn new(
        record_ctx: RecordCtx,
        freq_range: FreqRange,
        sample_rate_hz: f64,
        fft_size: usize,
        value_kind: BinValueKind,
        power_ctx: PowerCtx,
        bins: Vec<f32>,
    ) -> Result<Self, String> {
        if bins.len() != fft_size {
            return Err(format!("bins len {} != fft_size {}", bins.len(), fft_size));
        }

        if fft_size == 0 {
            return Err("fft_size must be > 0".to_string());
        }

        if freq_range.upper_edge_hz <= freq_range.lower_edge_hz {
            return Err("upper_edge_hz must be > lower_edge_hz".to_string());
        }

        if power_ctx.peak_bin >= bins.len() {
            return Err(format!(
                "peak_bin {} out of range {}",
                power_ctx.peak_bin,
                bins.len()
            ));
        }

        let bin_width_hz =
            (freq_range.upper_edge_hz - freq_range.lower_edge_hz) / fft_size as f64;

        Ok(Self {
            record_ctx,
            freq_range,
            value_kind,
            power_ctx,
            bin_width_hz,
            sample_rate_hz,
            fft_size,
            bins,
        })
    }

    pub fn hz_for_bin(&self, bin: usize) -> f64 {
        self.freq_range.lower_edge_hz + (bin as f64 * self.bin_width_hz)
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IqChunkMessage {
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
        samples: &[Complex32],
    ) -> Self {
        Self {
            record_ctx,
            center_hz,
            sample_rate_hz,
            sample_format: IqSampleFormat::ComplexF32,
            sample_count: samples.len(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IqCaptureMode {
    Off,
    Stream,
    Snapshot,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeEvent {
    Status(StatusMessage),
    Record(RecordMessage),
    Waterfall(Box<WaterfallMessage>),
    IqChunk(IqChunkMessage),
}

impl StatusMessage {
    pub fn new(
        edge_id: String,
        source_id: String,
        status: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            r#type: MessageKind::Status.as_str().to_string(),
            edge_id,
            source_id,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            status: status.into(),
            message: message.into(),
        }
    }
}


#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub avg_power: f32,
    pub noise_floor: f32,
    pub center_hz: f64,
    pub peak_bin: usize,
    pub peak_power: f32,
    pub estimated_peak_hz: f64,
    pub lower_edge_hz: f64,
    pub upper_edge_hz: f64,
    pub spectrum: Vec<f32>,
}