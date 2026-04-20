use num_complex::Complex32;
use serde::{Deserialize, Serialize};
use crate::capture::stream::RtlStream;
use crate::scanner::config::DEFAULT_SAMPLE_TIMEOUT;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettleStrategy {
    SleepOnly { millis: u64 },
    FlushBuffers { count: i64, timeout_us: i64 },
    SleepAndFlush {
        millis: u64,
        flush_count: i64,
        timeout_us: i64,
    },
}

impl Default for SettleStrategy {
    fn default() -> Self {
        Self::SleepAndFlush {
            millis: 5,
            flush_count: 2,
            timeout_us: 250_000,
        }
    }
}

pub fn dwell_capture(
    stream: &mut RtlStream,
    freq: f64,
    dwell_ms: u64,
    settle: &SettleStrategy,
) -> Result<Vec<Complex32>, Box<dyn std::error::Error>> {
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
        samples.extend(chunk.into_iter().map(|s| s.to_complex()));
    }

    Ok(samples)
}
