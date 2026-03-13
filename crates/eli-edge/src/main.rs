use soapysdr::{Device, Direction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Discover devices
    let results = soapysdr::enumerate("")?;

    if results.is_empty() {
        println!("No SDR devices found");
        return Ok(());
    }

    println!("Found {} device(s)\n", results.len());

    for (i, args) in results.into_iter().enumerate() {
        println!("Device {}:", i);

        for (k, v) in args.iter() {
            println!("  {}: {}", k, v);
        }

        // Open the device
        let dev = Device::new(args)?;

        println!("  Driver: {}", dev.driver_key()?);
        println!("  Hardware: {}", dev.hardware_key()?);

        let rx_channels = dev.num_channels(Direction::Rx)?;
        println!("  RX channels: {}", rx_channels);

        if rx_channels > 0 {
            let rates = dev.sample_rate(Direction::Rx, 0)?;
            println!("  Sample rates: {:?}", rates);

            let ranges = dev.frequency_range(Direction::Rx, 0)?;
            println!("  Frequency ranges: {:?}", ranges);
        }

        println!();
    }

    Ok(())
}
