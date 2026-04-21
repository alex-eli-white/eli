use std::collections::HashMap;
use soapysdr::{Device, Direction};
use soapysdr_sys::SoapySDRRange;
// use crate::router::genesis::discovery::{DeviceDiscovery, DiscoveredDevice};
// use crate::router::registries::device_registry::DeviceDescriptor;
// use crate::types::DeviceCapabilities;
use crate::{RouterError, RouterResult};
use crate::router::registries::reg_vanilla::{DeviceBackend, DeviceCapabilities, DeviceDescriptor, DeviceDiscovery};

pub struct RtlSdrDevice;

impl DeviceDiscovery for RtlSdrDevice {

    type RangesType = SoapySDRRange;
    fn discover(&self) -> RouterResult<Vec<DeviceDescriptor<Self::RangesType>>> {
        let results = soapysdr::enumerate("driver=rtlsdr")?;

        let mut devices = Vec::new();

        for args in results {
            let label = args.get("label").map(|s| s.to_string());
            let manufacturer = args.get("manufacturer").map(|s| s.to_string());
            let product = args.get("product").map(|s| s.to_string());
            let serial = args.get("serial").map(|s| s.to_string());
            let tuner = args.get("tuner").map(|s| s.to_string());


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




            let backend = DeviceBackend::RtlSdr;
            let capabilities = DeviceCapabilities {
                rx_channels,
                tx_channels:None,
                full_duplex: false,
                tuner,
                sample_rate: current_sample_rate,
                frequency_ranges: Some(frequency_ranges),
            };

            let device_descriptor = DeviceDescriptor {
                device_id : serial,
                product,
                manufacturer,
                backend,
                capabilities,
                label,
            };
        }

        Ok(devices)
    }
}


