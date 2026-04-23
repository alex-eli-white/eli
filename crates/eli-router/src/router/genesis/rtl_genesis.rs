use soapysdr::{Device, Direction};
use soapysdr_sys::SoapySDRRange;
use eli_protocol::router_vanilla::device_vanilla::{DeviceBackend, DeviceCapabilities, DeviceDescriptor, DeviceDiscovery};
use eli_protocol::router_vanilla::result_vanilla::RouterResult;


pub struct RtlSdrDiscovery;

impl DeviceDiscovery for RtlSdrDiscovery {
    type RangesType = Vec<SoapySDRRange>;

    fn discover(&self) -> RouterResult<Vec<DeviceDescriptor<Self::RangesType>>> {
        let results = soapysdr::enumerate("driver=rtlsdr")?;
        let mut devices = Vec::new();

        for args in results {
            let label = args.get("label").map(|s| s.to_string());
            let manufacturer = args.get("manufacturer").map(|s| s.to_string());
            let product = args.get("product").map(|s| s.to_string());
            let serial_number = args.get("serial").map(|s| s.to_string());
            let tuner = args.get("tuner").map(|s| s.to_string());

            let dev = Device::new(args)?;
            let rx_channels = dev.num_channels(Direction::Rx)?;

            if rx_channels == 0 {
                continue;
            }

            let sample_rate = Some(dev.sample_rate(Direction::Rx, 0)?);
            let frequency_ranges = Some(dev.frequency_range(Direction::Rx, 0)?);

            devices.push(DeviceDescriptor {
                backend: DeviceBackend::Rtl,
                serial_number,
                product,
                label,
                manufacturer,
                capabilities: DeviceCapabilities {
                    rx_channels,
                    tx_channels: None,
                    full_duplex: false,
                    tuner,
                    sample_rate,
                    frequency_ranges,
                },
            });
        }

        Ok(devices)
    }
}
