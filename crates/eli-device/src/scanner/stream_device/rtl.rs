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
    pub rx_channel_cnt: usize,
    pub frequency_ranges: Vec<SoapySDRRange>,
}

impl RtlDevice {
    pub fn new(serial_number: &str) -> EdgeResult<Self> {
        let serial_number = format!("serial={}", serial_number);
        let device = open_rtlsdr_by_serial(&serial_number)?;

        Ok(device)
    }

}

// pub fn get_rtlsdr_devices(serial_number: &str) -> EdgeResult<Vec<RtlDevice>> {
//     let results = soapysdr::enumerate(serial_number)?;
//
//     let mut devices = Vec::new();
//
//     for args in results {
//
//
//
//         let dev = Device::new(args)?;
//
//         let rx_channel = dev.num_channels(Direction::Rx)?;
//         let rx = dev.rx_stream(&[rx_channel])?;
//
//         let current_sample_rate = if rx_channel > 0 {
//             Some(dev.sample_rate(Direction::Rx, rx_channel)?)
//         } else {
//             None
//         };
//
//         let frequency_ranges = if rx_channel > 0 {
//             dev.frequency_range(Direction::Rx, rx_channel)?
//         } else {
//             Vec::new()
//         };
//
//         devices.push(RtlDevice {
//             device: dev.clone(),
//             stream: rx,
//             current_sample_rate,
//             channels: rx_channel,
//             frequency_ranges,
//             scratch: vec![Complex32::new(0.0, 0.0); 1024],
//         });
//     }
//
//     Ok(devices)
// }


pub fn open_rtlsdr_by_serial(serial_number: &str) -> EdgeResult<RtlDevice> {
    let results = soapysdr::enumerate(serial_number)?;

    let args = results
        .into_iter()
        .next()
        .ok_or_else(|| EdgeError::msg(format!("No RTL-SDR found for serial: {serial_number}")))?;

    let dev = Device::new(args)?;

    let rx_channels = dev.num_channels(Direction::Rx)?;
    if rx_channels == 0 {
        return Err(EdgeError::msg(format!(
            "Device with serial {serial_number} has no RX channels"
        )));
    }

    let channel = 0;
    let stream = dev.rx_stream(&[channel])?;
    let current_sample_rate = Some(dev.sample_rate(Direction::Rx, channel)?);
    let frequency_ranges = dev.frequency_range(Direction::Rx, channel)?;

    Ok(RtlDevice {
        device: dev,
        stream,
        current_sample_rate,
        rx_channel_cnt: rx_channels,
        frequency_ranges,
        scratch: vec![Complex32::new(0.0, 0.0); 1024],
    })
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