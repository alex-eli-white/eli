mod algorithms;
mod mixing;

use std::f64::consts::PI;

use crate::mixing::{generate_signal, mix_signal};

fn main() {
    let sample_rate = 1_000.0;
    let rf_freq = 200.0;
    let lo_freq = 200.0;
    let n_samples = 50;

    let rf_signal = generate_signal(rf_freq, sample_rate, n_samples);

    let baseband_signal = mix_signal(&rf_signal, lo_freq, sample_rate);

    println!("First few RF samples:");
    for v in rf_signal.iter().take(10) {
        println!("{:.4}", v);
    }

    println!("\nFirst few mixed samples:");
    for v in baseband_signal.iter().take(10) {
        println!("{:.4}", v);
    }
}

fn generate_sinusoid(freq: f64, amp: f64, phase: f64, fs: f64, duration: f64) -> Vec<f64> {
    let samples = (duration * fs) as usize;

    let mut data = Vec::with_capacity(samples);

    for n in 0..samples {
        let n = n as f64;

        let value = amp * (2.0 * std::f64::consts::PI * (freq / fs) * n + phase).sin();

        data.push(value);
    }

    data
}

fn multiply_signals(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b.iter()).map(|(x, m)| x * m).collect()
}
