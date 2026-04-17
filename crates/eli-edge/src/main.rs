mod capture;
mod dsp;

use dsp::dafft::compute_fft;

use capture::discovery::{discover_rtlsdr_devices, open_first_rtlsdr};
use capture::stream::RtlStream;

use num_complex::Complex32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_rtlsdr_devices()?;

    if devices.is_empty() {
        println!("No RTL-SDR devices found");
        return Ok(());
    }

    println!("Found {} RTL-SDR device(s)\n", devices.len());

    for (idx, dev) in devices.iter().enumerate() {
        println!("Device {idx}:");
        println!("  driver: {}", dev.driver);
        println!("  label: {:?}", dev.label);
        println!("  manufacturer: {:?}", dev.manufacturer);
        println!("  product: {:?}", dev.product);
        println!("  serial: {:?}", dev.serial);
        println!("  tuner: {:?}", dev.tuner);
        println!("  rx_channels: {}", dev.rx_channels);
        println!("  current_sample_rate: {:?}", dev.current_sample_rate);
        println!("  frequency_ranges: {:?}", dev.frequency_ranges);
        println!();
    }

    let dev = open_first_rtlsdr()?;
    let mut stream = RtlStream::open(dev, 100_000_000.0, 2_048_000.0)?;

    println!("Configured:");
    println!("  sample rate: {}", stream.current_sample_rate()?);
    println!("  frequency:   {}", stream.current_frequency()?);

    stream.activate()?;

    let fft_size = 4096;

    for chunk_idx in 0..5 {
        let samples = stream.read_samples(1_000_000)?;

        if samples.len() < fft_size {
            continue;
        }

        let chunk = &samples[..fft_size];

        let complex = chunk.iter().map(|s| Complex32::new(s.i, s.q)).collect::<Vec<_>>();

        let centered = remove_dc(&complex);
        let spectrum = compute_fft(&centered);
        


        let (max_idx, max_val) = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        println!("\nchunk {chunk_idx}:");
        println!("  strongest bin: {} (magnitude {:.4})", max_idx, max_val);
    }

    stream.deactivate()?;

    Ok(())
}



pub fn remove_dc(samples: &[Complex32]) -> Vec<Complex32> {
    let len = samples.len() as f32;

    let mean_i = samples.iter().map(|s| s.re).sum::<f32>() / len;
    let mean_q = samples.iter().map(|s| s.im).sum::<f32>() / len;

    samples
        .iter()
        .map(|s| Complex32::new(s.re - mean_i, s.im - mean_q))
        .collect()
}