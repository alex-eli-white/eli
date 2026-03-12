use std::f64::consts::PI;

/// Generate a sinusoidal signal
pub fn generate_signal(freq: f64, sample_rate: f64, n_samples: usize) -> Vec<f64> {
    (0..n_samples)
        .map(|n| {
            let t = n as f64 / sample_rate;
            (2.0 * PI * freq * t).cos()
        })
        .collect()
}

/// Mix (frequency shift) a signal using a local oscillator
pub fn mix_signal(signal: &[f64], lo_freq: f64, sample_rate: f64) -> Vec<f64> {
    signal
        .iter()
        .enumerate()
        .map(|(n, &x)| {
            let t = n as f64 / sample_rate;
            let lo = (2.0 * PI * lo_freq * t).cos();
            x * lo
        })
        .collect()
}
