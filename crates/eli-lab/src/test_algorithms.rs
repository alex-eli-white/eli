pub fn bandpass_sampling_range(fc: f64, bandwidth: f64, m: u32) -> Option<(f64, f64)> {
    let m = m as f64;

    let upper = (2.0 * fc - bandwidth) / m;
    let lower = (2.0 * fc + bandwidth) / (m + 1.0);

    if lower <= upper {
        Some((lower, upper))
    } else {
        None
    }
}

fn bandpass_sampling_ranges(fc: f64, bandwidth: f64, max_m: u32) {
    let nyquist_min = 2.0 * bandwidth;

    println!("fc = {} MHz, B = {} MHz\n", fc, bandwidth);

    for m in 1..=max_m {
        let m_f = m as f64;

        let upper = (2.0 * fc - bandwidth) / m_f;
        let lower = (2.0 * fc + bandwidth) / (m_f + 1.0);

        if lower <= upper && upper >= nyquist_min {
            let recommended = upper;

            println!(
                "m={}  valid fs: {:.2} MHz → {:.2} MHz   recommended ≈ {:.2} MHz",
                m, lower, upper, recommended
            );
        }
    }
}

fn fs_at_quarter_band(fc: f64, k: u32) -> f64 {
    assert!(k > 0, "k must be a positive integer");
    4.0 * fc / (2.0 * k as f64 - 1.0)
}

fn valid_fs_at_quarter_band(fc: f64, b: f64, max_k: u32) {
    let nyquist_min = 2.0 * b;

    for k in 1..=max_k {
        let fs = fs_at_quarter_band(fc, k);

        if fs >= nyquist_min {
            println!("k = {} -> valid fs = {:.4} MHz", k, fs);
        } else {
            println!("k = {} -> fs = {:.4} MHz (invalid: below 2B)", k, fs);
        }
    }
}

use num_complex::Complex;
use std::f64::consts::PI;

pub fn dft(samples: &[f64]) -> Vec<Complex<f64>> {
    let n = samples.len();
    let mut output = vec![Complex::new(0.0, 0.0); n];

    for m in 0..n {
        let mut sum = Complex::new(0.0, 0.0);

        for k in 0..n {
            let angle = -2.0 * PI * (m as f64) * (k as f64) / n as f64;

            let twiddle = Complex::new(angle.cos(), angle.sin());

            sum += samples[k] * twiddle;
        }

        output[m] = sum;
    }

    output
}

pub fn recursive_fft(x: &[Complex<f64>]) -> Vec<Complex<f64>> {
    let n = x.len();

    if n <= 1 {
        return x.to_vec();
    }

    let even: Vec<_> = x.iter().step_by(2).cloned().collect();
    let odd: Vec<_> = x.iter().skip(1).step_by(2).cloned().collect();

    let even_fft = recursive_fft(&even);
    let odd_fft = recursive_fft(&odd);

    let mut combined = vec![Complex::new(0.0, 0.0); n];

    for k in 0..n / 2 {
        let twiddle = Complex::from_polar(1.0, -2.0 * PI * k as f64 / n as f64) * odd_fft[k];

        combined[k] = even_fft[k] + twiddle;
        combined[k + n / 2] = even_fft[k] - twiddle;
    }

    combined
}

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
