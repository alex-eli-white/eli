use std::time::{SystemTime, UNIX_EPOCH};

use eli_edge::capture::discovery::{discover_rtlsdr_devices, open_first_rtlsdr};
use eli_edge::capture::stream::RtlStream;
use eli_edge::scanner::dwell_capture::{dwell_capture, SettleStrategy};
use eli_edge::scanner::fft_analysis::analyze;
use eli_edge::scanner::sweep_planner::{SweepPlanner, SweepPlannerConfig};
use eli_edge::scanner::vanilla::{ScanHit, SweepRecord};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_rtlsdr_devices()?;

    if devices.is_empty() {
        println!("No RTL-SDR devices found");
        return Ok(());
    }

    println!("Found {} RTL-SDR device(s)\n", devices.len());

    for (idx, dev) in devices.iter().enumerate() {
        println!("Device {idx}:");
        println!("  driver: {}", dev.driver);
        println!("  label: {:?}", dev.label);
        println!("  manufacturer: {:?}", dev.manufacturer);
        println!("  product: {:?}", dev.product);
        println!("  serial: {:?}", dev.serial);
        println!("  tuner: {:?}", dev.tuner);
        println!("  rx_channels: {}", dev.rx_channels);
        println!("  current_sample_rate: {:?}", dev.current_sample_rate);
        println!("  frequency_ranges: {:?}", dev.frequency_ranges);
        println!();
    }

    let sample_rate_hz = 2_048_000.0;
    let dwell_ms = 20;
    let hit_threshold_over_floor = 6.0;
    let fft_min_samples = 4096;

    let planner_cfg = SweepPlannerConfig {
        start_hz: 88_000_000.0,
        end_hz: 108_000_000.0,
        sample_rate_hz,
        usable_bandwidth_hz: 1_600_000.0,
        overlap_fraction: 0.25,
    };

    let hotspots = [
        (96_300_000.0, 2.0),
        (99_500_000.0, 1.5),
        (101_100_000.0, 1.5),
    ];

    let mut planner = SweepPlanner::new_priority(planner_cfg, &hotspots);

    println!("Sweep plan (priority order):");
    for point in planner.points() {
        println!(
            "  center={:.3} MHz, priority={:.2}, window={:.3}..{:.3} MHz",
            point.center_hz / 1e6,
            point.priority,
            point.lower_edge_hz / 1e6,
            point.upper_edge_hz / 1e6
        );
    }
    println!();

    let dev = open_first_rtlsdr()?;
    let mut stream = RtlStream::open(dev, planner.points()[0].center_hz, sample_rate_hz)?;

    println!("Configured:");
    println!("  sample rate: {}", stream.current_sample_rate()?);
    println!("  initial frequency: {}", stream.current_frequency()?);

    stream.activate()?;

    let mut records = Vec::<SweepRecord>::new();
    let mut hits = Vec::<ScanHit>::new();

    while let Some(point) = planner.pop_next() {
        let samples = dwell_capture(
            &mut stream,
            point.center_hz,
            dwell_ms,
            SettleStrategy::SleepAndFlush {
                millis: 5,
                flush_count: 2,
                timeout_us: 250_000,
            },
        )?;

        if samples.len() < fft_min_samples {
            continue;
        }

        let analysis = analyze(&samples[..fft_min_samples], point.center_hz, sample_rate_hz);
        let timestamp_ms = now_ms();
        let snr_like_db = power_ratio_db(analysis.peak_power, analysis.noise_floor);

        let record = SweepRecord {
            center_hz: point.center_hz,
            lower_edge_hz: analysis.lower_edge_hz,
            upper_edge_hz: analysis.upper_edge_hz,
            avg_power: analysis.avg_power,
            noise_floor: analysis.noise_floor,
            peak_power: analysis.peak_power,
            peak_bin: analysis.peak_bin,
            estimated_peak_hz: analysis.estimated_peak_hz,
            timestamp_ms,
        };

        println!(
            "center={:.3} MHz peak={:.3} MHz avg={:.3} floor={:.3} peak={:.3} snr≈{:.2} dB",
            record.center_hz / 1e6,
            record.estimated_peak_hz / 1e6,
            record.avg_power,
            record.noise_floor,
            record.peak_power,
            snr_like_db,
        );

        if snr_like_db >= hit_threshold_over_floor {
            hits.push(ScanHit {
                dwell_center_hz: point.center_hz,
                estimated_peak_hz: analysis.estimated_peak_hz,
                avg_power: analysis.avg_power,
                noise_floor: analysis.noise_floor,
                peak_bin: analysis.peak_bin,
                peak_power: analysis.peak_power,
                snr_like_db,
                timestamp_ms,
            });

            planner.reprioritize_near(analysis.estimated_peak_hz, 0.75, 1_500_000.0);
        }

        records.push(record);
    }

    stream.deactivate()?;

    println!("\nCompleted {} dwells", records.len());
    println!("Detected {} hits", hits.len());

    for hit in &hits {
        println!(
            "  HIT center={:.3} MHz peak≈{:.3} MHz floor={:.3} peak={:.3} snr≈{:.2} dB",
            hit.dwell_center_hz / 1e6,
            hit.estimated_peak_hz / 1e6,
            hit.noise_floor,
            hit.peak_power,
            hit.snr_like_db,
        );
    }

    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn power_ratio_db(peak: f32, floor: f32) -> f32 {
    let safe_peak = peak.max(1e-12);
    let safe_floor = floor.max(1e-12);
    20.0 * (safe_peak / safe_floor).log10()
}
