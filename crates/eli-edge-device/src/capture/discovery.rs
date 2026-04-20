use std::{collections::HashMap, fmt::Debug};

use soapysdr::{Args, Device, Direction};
use crate::scanner::EdgeResult;

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub args: HashMap<String, String>,
    pub driver: String,
    pub label: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub tuner: Option<String>,
    pub rx_channels: usize,
    pub current_sample_rate: Option<f64>,
    pub frequency_ranges: Vec<soapysdr::Range>,
}

pub fn discover_rtlsdr_devices() -> EdgeResult<Vec<DiscoveredDevice>> {
    let results = soapysdr::enumerate("driver=rtlsdr")?;

    let mut devices = Vec::new();

    for args in results {
        let label = args.get("label").map(|s| s.to_string());
        let manufacturer = args.get("manufacturer").map(|s| s.to_string());
        let product = args.get("product").map(|s| s.to_string());
        let serial = args.get("serial").map(|s| s.to_string());
        let tuner = args.get("tuner").map(|s| s.to_string());

        let arg_map = turn_args_hashmap(&args);

        let dev = Device::new(args)?;

        let driver = dev.driver_key()?;
        let rx_channels = dev.num_channels(Direction::Rx)?;

        let current_sample_rate = if rx_channels > 0 {
            Some(dev.sample_rate(Direction::Rx, 0)?)
        } else {
            None
        };

        let frequency_ranges = if rx_channels > 0 {
            dev.frequency_range(Direction::Rx, 0)?
        } else {
            Vec::new()
        };

        devices.push(DiscoveredDevice {
            driver,
            label,
            manufacturer,
            product,
            serial,
            tuner,
            rx_channels,
            current_sample_rate,
            frequency_ranges,
            args: arg_map,
        });
    }

    Ok(devices)
}

pub fn open_first_rtlsdr() -> EdgeResult<Device> {
    let results = soapysdr::enumerate("driver=rtlsdr")?;
    let args = results
        .into_iter()
        .next()
        .ok_or("No RTL-SDR devices found")?;
    let dev = Device::new(args)?;
    Ok(dev)
}

fn turn_args_hashmap(args: &Args) -> HashMap<String, String> {
    let mut outmap = HashMap::new();

    for (key, value) in args.iter() {
        let key = key.to_string();
        let value = value.to_string();

        outmap.insert(key, value);
    }

    outmap
}


pub fn open_rtlsdr_by_index(index: usize) -> EdgeResult<Device> {
    let devices = discover_rtlsdr_devices()?;

    let info = devices
        .get(index)
        .ok_or_else(|| format!("no RTL-SDR device at index {}", index))?;

    let mut args = soapysdr::Args::new();
    args.set("driver", "rtlsdr");

    if let Some(serial) = &info.serial {
        args.set("serial", String::from(serial));
    }

    let dev = soapysdr::Device::new(args)?;
    Ok(dev)
}