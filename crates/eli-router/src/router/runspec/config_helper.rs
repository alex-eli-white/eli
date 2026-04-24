use eli_protocol::edge_vanilla::scanner::config_vanilla::{FixedModeConfig, ScannerConfig, ScannerMode, SweepModeConfig};
use eli_protocol::edge_vanilla::scanner::dwell_vanilla::SettleStrategy;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::IqCaptureMode;
use eli_protocol::edge_vanilla::scanner::sweep_vanilla::{SweepCoverage, SweepExecution, SweepPolicy};
use crate::router::registries::worker_registry::now_ms;

pub fn fm_sweep_config(worker_id: &str) -> ScannerConfig {
    ScannerConfig {
        edge_id: worker_id.to_string(),
        source_id: "rtl-sdr-0".to_string(),
        timestamp_ms: now_ms(),
        sample_rate_hz: 2_048_000.0,
        settle: SettleStrategy::SleepAndFlush {
            millis: 5,
            flush_count: 2,
            timeout_us: 250_000,
        },
        mode: ScannerMode::Sweep(SweepModeConfig {
            coverage: SweepCoverage {
                start_hz: 88_000_000.0,
                end_hz: 108_000_000.0,
                sample_rate_hz: 2_048_000.0,
                usable_bandwidth_hz: 1_800_000.0,
                overlap_fraction: 0.15,
            },
            execution: SweepExecution {
                dwell_ms: 20,
                settle_ms: 5,
                flush_count: 2,
            },
            policy: SweepPolicy::Sequential,
            hotspots: vec![],
            fft_min_samples: 2048,
        }),
    }
}

pub fn fixed_config(worker_id: &str, center_hz: f64) -> ScannerConfig {
    ScannerConfig {
        edge_id: worker_id.to_string(),
        source_id: "rtl-sdr-0".to_string(),
        timestamp_ms: now_ms(),
        sample_rate_hz: 2_048_000.0,
        settle: SettleStrategy::SleepAndFlush {
            millis: 5,
            flush_count: 2,
            timeout_us: 250_000,
        },
        mode: ScannerMode::Fixed(FixedModeConfig {
            center_hz,
            sample_rate_hz: 2_048_000.0,
            dwell_ms: 20,
            fft_min_samples: 2048,
            settle: SettleStrategy::SleepAndFlush {
                millis: 5,
                flush_count: 2,
                timeout_us: 250_000,
            },
            iq_capture: IqCaptureMode::Off,
            iq_chunk_samples: 0,
        }),
    }
}

pub fn idle_config(worker_id: &str) -> ScannerConfig {
    ScannerConfig {
        edge_id: worker_id.to_string(),
        source_id: "rtl-sdr-0".to_string(),
        timestamp_ms: now_ms(),
        sample_rate_hz: 2_048_000.0,
        settle: SettleStrategy::SleepAndFlush {
            millis: 5,
            flush_count: 2,
            timeout_us: 250_000,
        },
        mode: ScannerMode::Idle,
    }
}