use std::sync::Arc;
use num_complex::Complex32;
use serde::{Deserialize, Serialize};
use eli_protocol::edge_vanilla::scanner::config_vanilla::DEFAULT_SAMPLE_TIMEOUT;
use eli_protocol::edge_vanilla::scanner::dwell_vanilla::SettleStrategy;
use crate::EdgeResult;
use crate::scanner::stream_device::stream_vanilla::{DeviceStream};

pub fn dwell_capture(
    stream: &mut dyn DeviceStream,
    freq: f64,
    dwell_ms: u64,
    settle: &SettleStrategy,
) -> EdgeResult<Vec<Complex32>> {
    stream.set_frequency(freq)?;

    match settle {
        SettleStrategy::SleepOnly { millis } => {

            std::thread::sleep(std::time::Duration::from_millis(*millis));
        }
        SettleStrategy::FlushBuffers { count, timeout_us } => {

            stream.discard_buffers(*count, *timeout_us)?;
        }
        SettleStrategy::SleepAndFlush {
            millis,
            flush_count,
            timeout_us,
        } => {
            std::thread::sleep(std::time::Duration::from_millis(*millis));
            stream.discard_buffers(*flush_count, *timeout_us)?;
        }
    }

    let mut samples = Vec::new();
    let start = std::time::Instant::now();

    while start.elapsed().as_millis() < dwell_ms as u128 {
        let chunk = stream.read_samples(DEFAULT_SAMPLE_TIMEOUT)?;
        samples.extend(chunk);
    }

    Ok(samples)
}

