use num_complex::Complex32;
use soapysdr::{Device, Direction};
use eli_protocol::edge_vanilla::scanner::config_vanilla::DEFAULT_SAMPLE_TIMEOUT;
use eli_protocol::edge_vanilla::scanner::dwell_vanilla::SettleStrategy;
use crate::capture::stream::{IqSample};
use crate::edge_error::EdgeError;
use crate::EdgeResult;
use crate::scanner::dwell_capture::{dwell_capture};
use crate::scanner::streams::stream_vanilla::DeviceStream;



pub struct RtlStream {
    device: Device,
    stream: soapysdr::RxStream<Complex32>,
    scratch: Vec<Complex32>,
}

impl DeviceStream for RtlStream {
    fn set_frequency(&mut self, freq_hz: f64) -> Result<(), EdgeError> {
        self.set_frequency(freq_hz)?;
        Ok(())
    }

    fn capture_dwell(
        &mut self,
        center_hz: f64,
        dwell_ms: u64,
        settle: &SettleStrategy,
    ) -> Result<Vec<Complex32>, EdgeError> {
        dwell_capture(self, center_hz, dwell_ms, settle)
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
            out.push(IqSample {
                i: sample.re,
                q: sample.im,
            }.to_complex());
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