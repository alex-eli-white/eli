use num_complex::Complex32;
use soapysdr::{Device, Direction};
use soapysdr_sys::SoapySDRRange;

use crate::edge_error::EdgeError;
use crate::EdgeResult;
use crate::scanner::stream_device::stream_vanilla::DeviceStream;



pub struct RtlDevice {
    pub device: Device,
    pub stream: soapysdr::RxStream<Complex32>,
    pub scratch: Vec<Complex32>,
    pub current_sample_rate: Option<f64>,
    pub frequency_ranges: Vec<SoapySDRRange>,
}

impl RtlDevice {
    pub fn new(serial_number: &str) -> EdgeResult<Self> {
        let search = format!("serial={}", serial_number);
        let devices = get_rtlsdr_devices(&search)?;

        let rtl_device = devices.into_iter().take(1).next()
            .ok_or(EdgeError::RtlSdrDeviceNotFound(serial_number.to_string()))?;

        Ok(rtl_device)
    }

}

pub fn get_rtlsdr_devices(serial_number: &str) -> EdgeResult<Vec<RtlDevice>> {
    let results = soapysdr::enumerate(serial_number)?;

    let mut devices = Vec::new();

    for args in results {



        let dev = Device::new(args)?;

        let rx_channel = dev.num_channels(Direction::Rx)?;
        let rx = dev.rx_stream(&[rx_channel])?;

        let current_sample_rate = if rx_channel > 0 {
            Some(dev.sample_rate(Direction::Rx, rx_channel)?)
        } else {
            None
        };

        let frequency_ranges = if rx_channel > 0 {
            dev.frequency_range(Direction::Rx, rx_channel)?
        } else {
            Vec::new()
        };

        devices.push(RtlDevice {
            device: dev.clone(),
            stream: rx,
            current_sample_rate,
            frequency_ranges,
            scratch: vec![Complex32::new(0.0, 0.0); 1024],
        });
    }

    Ok(devices)
}

impl DeviceStream for RtlDevice {
    fn set_frequency(&mut self, freq_hz: f64) -> EdgeResult<()> {
        self.device.set_frequency(Direction::Rx, 0, freq_hz, ())?;
        Ok(())
    }



    fn discard_buffers(&mut self, count: i64, timeout_us: i64) -> Result<(), EdgeError> {
        for _ in 0..count {
            let _ = self.read_samples(timeout_us)?;
        }

        Ok(())
    }

    fn read_samples(&mut self, timeout_us: i64) -> Result<Vec<Complex32>, EdgeError> {
        let mut buffers = [&mut self.scratch[..]];
        let count = self.stream.read(&mut buffers, timeout_us)?;

        let mut out = Vec::with_capacity(count);

        for sample in &self.scratch[..count] {
            out.push(*sample);
        }

        Ok(out)
    }

    fn activate(&mut self) -> EdgeResult<()> {
        self.stream.activate(None)?;
        Ok(())
    }

    fn deactivate(&mut self) -> EdgeResult<()> {
        self.stream.deactivate(None)?;
        Ok(())
    }

    fn current_sample_rate(&self) -> EdgeResult<f64> {
        Ok(self.device.sample_rate(Direction::Rx, 0)?)
    }

    fn current_frequency(&self) -> EdgeResult<f64> {
        Ok(self.device.frequency(Direction::Rx, 0)?)
    }
}