use serde::{Deserialize, Serialize};

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