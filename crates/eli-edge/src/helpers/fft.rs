use num_complex::Complex32;
use rustfft::FftPlanner;

pub fn compute_fft(samples: &[Complex32]) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(samples.len());

    let mut buffer = samples.to_vec();
    fft.process(&mut buffer);

    // magnitude spectrum
    buffer
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect()
}