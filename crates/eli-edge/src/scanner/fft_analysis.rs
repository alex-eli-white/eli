use num_complex::Complex32;

use crate::helpers::{dc::remove_dc, fft::compute_fft};

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
}

pub fn analyze(samples: &[Complex32], center_hz: f64, sample_rate_hz: f64) -> AnalysisResult {
    let centered = remove_dc(samples);
    let spectrum = compute_fft(&centered);

    let avg_power = spectrum.iter().sum::<f32>() / spectrum.len() as f32;
    let noise_floor = percentile(&spectrum, 0.50);

    let (peak_bin, peak_power) = spectrum
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.partial_cmp(b.1)
                .unwrap_or(std::cmp::Ordering::Less)
        })
        .map(|(i, v)| (i, *v))
        .unwrap_or((0, 0.0));

    let bin_hz = sample_rate_hz / spectrum.len() as f64;
    let half_span_hz = sample_rate_hz / 2.0;
    let lower_edge_hz = center_hz - half_span_hz;
    let upper_edge_hz = center_hz + half_span_hz;
    let estimated_peak_hz = lower_edge_hz + (peak_bin as f64 * bin_hz);

    AnalysisResult {
        avg_power,
        center_hz,
        noise_floor,
        peak_bin,
        peak_power,
        estimated_peak_hz,
        lower_edge_hz,
        upper_edge_hz,
    }
}

fn percentile(values: &[f32], q: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Less));

    let q = q.clamp(0.0, 1.0);
    let idx = ((sorted.len() - 1) as f32 * q).round() as usize;
    sorted[idx]
}
