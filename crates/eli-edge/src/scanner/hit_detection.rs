use crate::scanner::fft_analysis::AnalysisResult;

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
            min_peak_power: 1.0,
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

pub fn estimate_snr_db(peak_power: f32, noise_floor: f32) -> f32 {
    let floor = noise_floor.max(1e-12);
    let ratio = (peak_power / floor).max(1e-12);
    10.0 * ratio.log10()
}

pub fn detect_hit(
    cfg: &HitDetectorConfig,
    source_id: &str,
    timestamp_ms: u64,
    analysis: &AnalysisResult,
    spectrum_len: usize,
) -> Option<Hit> {
    if spectrum_len == 0 {
        return None;
    }

    let left_guard = cfg.edge_exclusion_bins;
    let right_guard = spectrum_len.saturating_sub(cfg.edge_exclusion_bins);

    if analysis.peak_bin < left_guard || analysis.peak_bin >= right_guard {
        return None;
    }

    if analysis.peak_power < cfg.min_peak_power {
        return None;
    }

    let snr_db = estimate_snr_db(analysis.peak_power, analysis.noise_floor);

    if snr_db < cfg.min_snr_db {
        return None;
    }

    Some(Hit {
        source_id: source_id.to_string(),
        center_hz: (analysis.lower_edge_hz + analysis.upper_edge_hz) / 2.0,
        peak_hz: analysis.estimated_peak_hz,
        lower_edge_hz: analysis.lower_edge_hz,
        upper_edge_hz: analysis.upper_edge_hz,
        peak_bin: analysis.peak_bin,
        peak_power: analysis.peak_power,
        noise_floor: analysis.noise_floor,
        avg_power: analysis.avg_power,
        snr_db,
        timestamp_ms,
    })
}