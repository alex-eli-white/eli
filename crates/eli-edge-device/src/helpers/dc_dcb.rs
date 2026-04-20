use num_complex::Complex32;
use crate::POWER_EPSILON;

pub fn remove_dc(samples: &[Complex32]) -> Vec<Complex32> {
    let len = samples.len() as f32;

    let mean_i = samples.iter().map(|s| s.re).sum::<f32>() / len;
    let mean_q = samples.iter().map(|s| s.im).sum::<f32>() / len;

    samples
        .iter()
        .map(|s| Complex32::new(s.re - mean_i, s.im - mean_q))
        .collect()
}

pub fn power_to_db(power: f32) -> f32 {
    10.0 * f32::log10(power.max(POWER_EPSILON))
}