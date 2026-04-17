mod capture;

use capture::discovery::{discover_rtlsdr_devices, open_first_rtlsdr};
use capture::stream::RtlStream;

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

    for chunk_idx in 0..5 {
        let samples = stream.read_samples(1_000_000)?;

        println!("\nchunk {chunk_idx}: {} IQ samples", samples.len());

        for (i, sample) in samples.iter().take(8).enumerate() {
            println!("  sample {:>2}: I={:+.4}, Q={:+.4}", i, sample.i, sample.q);
        }
    }

    stream.deactivate()?;

    Ok(())
}
