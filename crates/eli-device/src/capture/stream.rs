use num_complex::Complex32;
use soapysdr::{Device, Direction, Format, StreamSample};
use crate::EdgeResult;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IqSample {
    pub i: f32,
    pub q: f32,
}

impl IqSample {
    pub fn to_complex(&self) -> Complex32 {
        Complex32::new(self.i, self.q)
    }
    pub fn new(i: f32, q: f32) -> Self {
        Self { i, q }
    }
}


unsafe impl StreamSample for IqSample {
    const STREAM_FORMAT: Format = Format::CF32;
}

pub struct RtlStream {
    device: Device,
    stream: soapysdr::RxStream<IqSample>,
    scratch: Vec<IqSample>,
}

impl RtlStream {
    pub fn open(
        mut device: Device,
        center_hz: f64,
        sample_rate: f64,
    ) -> EdgeResult<Self> {
        device.set_sample_rate(Direction::Rx, 0, sample_rate)?;
        device.set_frequency(Direction::Rx, 0, center_hz, ())?;

        let stream = device.rx_stream::<IqSample>(&[0])?;

        Ok(Self {
            device,
            stream,
            scratch: vec![IqSample::new(0.0, 0.0); 16_384],
        })
    }

    pub fn activate(&mut self) -> EdgeResult<()> {
        self.stream.activate(None)?;
        Ok(())
    }

    pub fn deactivate(&mut self) -> EdgeResult<()> {
        self.stream.deactivate(None)?;
        Ok(())
    }

    pub fn current_sample_rate(&self) -> EdgeResult<f64> {
        Ok(self.device.sample_rate(Direction::Rx, 0)?)
    }

    pub fn current_frequency(&self) -> EdgeResult<f64> {
        Ok(self.device.frequency(Direction::Rx, 0)?)
    }

    pub fn read_samples(
        &mut self,
        timeout_us: i64,
    ) -> EdgeResult<Vec<IqSample>> {
        let mut buffers = [&mut self.scratch[..]];
        let count = self.stream.read(&mut buffers, timeout_us)?;

        Ok(self.scratch[..count].to_vec())
    }

    pub fn discard_buffers(
        &mut self,
        count: i64,
        timeout_us: i64,
    ) -> EdgeResult<()> {
        for _ in 0..count {
            let _ = self.read_samples(timeout_us)?;
        }

        Ok(())
    }

    pub fn set_frequency(&mut self, freq: f64) -> EdgeResult<()> {
        self.device.set_frequency(Direction::Rx, 0, freq, ())?;
        Ok(())
    }
}
