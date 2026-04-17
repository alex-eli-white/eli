use num_complex::Complex32;
use soapysdr::{Device, Direction};

#[derive(Debug, Clone, Copy)]
pub struct IqSample {
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
    ) -> Result<Self, Box<dyn std::error::Error>> {
        device.set_sample_rate(Direction::Rx, 0, sample_rate)?;
        device.set_frequency(Direction::Rx, 0, center_hz, ())?;

        let stream = device.rx_stream::<Complex32>(&[0])?;

        Ok(Self {
            device,
            stream,
            scratch: vec![Complex32::new(0.0, 0.0); 16_384],
        })
    }

    pub fn activate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.activate(None)?;
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.deactivate(None)?;
        Ok(())
    }

    pub fn current_sample_rate(&self) -> Result<f64, Box<dyn std::error::Error>> {
        Ok(self.device.sample_rate(Direction::Rx, 0)?)
    }

    pub fn current_frequency(&self) -> Result<f64, Box<dyn std::error::Error>> {
        Ok(self.device.frequency(Direction::Rx, 0)?)
    }

    pub fn read_samples(
        &mut self,
        timeout_us: i64,
    ) -> Result<Vec<IqSample>, Box<dyn std::error::Error>> {
        let mut buffers = [&mut self.scratch[..]];
        let result = self.stream.read(&mut buffers, timeout_us)?;

        let iq_pairs = result / 2;

        let mut out = Vec::with_capacity(iq_pairs);

        for idx in 0..iq_pairs {
            let i_raw = self.scratch[idx * 2];
            let q_raw = self.scratch[idx * 2 + 1];

            let i = (i_raw.re as f32 - 127.5) / 128.0;
            let q = (q_raw.im as f32 - 127.5) / 128.0;

            out.push(IqSample { i, q });
        }

        Ok(out)
    }
}
