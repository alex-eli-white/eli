use num_complex::Complex32;
use soapysdr::{Device, Direction};
use crate::scanner::EdgeResult;

#[derive(Debug, Clone, Copy)]
pub struct IqSample {
    pub i: f32,
    pub q: f32,
}

impl IqSample {
    pub fn to_complex(&self) -> Complex32 {
        Complex32::new(self.i, self.q)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ComplexSample {
    pub i: f32,
    pub q: f32,
}


pub struct RtlStream {
    device: Device,
    stream: soapysdr::RxStream<Complex32>,
    scratch: Vec<Complex32>,
}

impl RtlStream {
    pub fn open(
        mut device: Device,
        center_hz: f64,
        sample_rate: f64,
    ) -> EdgeResult<Self> {
        device.set_sample_rate(Direction::Rx, 0, sample_rate)?;
        device.set_frequency(Direction::Rx, 0, center_hz, ())?;

        let stream = device.rx_stream::<Complex32>(&[0])?;

        Ok(Self {
            device,
            stream,
            scratch: vec![Complex32::new(0.0, 0.0); 16_384],
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

        let mut out = Vec::with_capacity(count);

        for sample in &self.scratch[..count] {
            out.push(IqSample {
                i: sample.re,
                q: sample.im,
            });
        }

        Ok(out)
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
