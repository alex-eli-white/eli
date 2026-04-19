use rustfft::FftPlanner;
use num_complex::Complex32;

pub fn compute_fft(samples: &[Complex32]) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(samples.len());

    let mut buffer = samples.to_vec();
    fft.process(&mut buffer);

    let scale = 1.0 / samples.len() as f32;

    buffer
        .iter()
        .map(|c| {
            let re = c.re * scale;
            let im = c.im * scale;
            re * re + im * im
        })
        .collect()
}