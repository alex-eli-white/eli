use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use tower_http::cors::{Any, CorsLayer};
use axum::http::{Method, HeaderValue};

use eli_edge::capture::discovery::{discover_rtlsdr_devices, open_first_rtlsdr};
use eli_edge::capture::stream::RtlStream;
use eli_edge::scanner::dwell_capture::{dwell_capture, SettleStrategy};
use eli_edge::scanner::fft_analysis::analyze;
use eli_edge::scanner::hit_detection::{detect_hit, Hit, HitDetectorConfig};
use eli_edge::scanner::sweep_planner::{SweepPlanner, SweepCoverage};
use eli_edge::scanner::vanilla::SweepRecord;

#[derive(Clone)]
struct AppState {
    record_tx: broadcast::Sender<RecordMessage>,
    hit_tx: broadcast::Sender<HitMessage>,
    waterfall_tx: broadcast::Sender<WaterfallMessage>,
    scanner_running: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize)]
struct RecordMessage {
    #[serde(rename = "type")]
    kind: &'static str,
    source_id: String,
    timestamp_ms: u64,
    center_hz: f64,
    lower_edge_hz: f64,
    upper_edge_hz: f64,
    avg_power: f32,
    noise_floor: f32,
    peak_power: f32,
    peak_bin: usize,
    estimated_peak_hz: f64,
}

#[derive(Debug, Clone, Serialize)]
struct HitMessage {
    #[serde(rename = "type")]
    kind: &'static str,
    source_id: String,
    timestamp_ms: u64,
    center_hz: f64,
    peak_hz: f64,
    lower_edge_hz: f64,
    upper_edge_hz: f64,
    peak_bin: usize,
    peak_power: f32,
    noise_floor: f32,
    avg_power: f32,
    snr_db: f32,
}

#[derive(Debug, Clone, Serialize)]
struct WaterfallMessage {
    #[serde(rename = "type")]
    kind: &'static str,
    source_id: String,
    timestamp_ms: u64,
    center_hz: f64,
    lower_edge_hz: f64,
    upper_edge_hz: f64,
    bins: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct ScannerStatus {
    is_running: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let (record_tx, _) = broadcast::channel::<RecordMessage>(256);
    let (hit_tx, _) = broadcast::channel::<HitMessage>(256);
    let (waterfall_tx, _) = broadcast::channel::<WaterfallMessage>(64);

    let scanner_running = Arc::new(AtomicBool::new(true));

    let state = AppState {
        record_tx: record_tx.clone(),
        hit_tx: hit_tx.clone(),
        waterfall_tx: waterfall_tx.clone(),
        scanner_running: scanner_running.clone(),
    };

    println!("about to build app");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/ws/records", get(ws_records_handler))
        .route("/ws/hits", get(ws_hits_handler))
        .route("/ws/waterfall", get(ws_waterfall_handler))
        .route("/api/scanner/status", get(scanner_status_handler))
        .route("/api/scanner/start", post(scanner_start_handler))
        .route("/api/scanner/stop", post(scanner_stop_handler))
        .route("/healthz", get(|| async { "ok" }))
        .layer(cors)
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:9001".parse()?;

    println!("about to bind listener on {addr}");
    let listener = TcpListener::bind(addr).await?;
    println!("listener bound successfully");

    println!("about to spawn scanner thread");
    tokio::task::spawn_blocking(move || {
        println!("scanner thread entered");
        if let Err(err) = run_scan_loop(record_tx, hit_tx, waterfall_tx, scanner_running) {
            eprintln!("scanner loop exited with error: {err}");
        }
        println!("scanner thread exited");
    });

    println!("about to serve axum");
    axum::serve(listener, app).await?;
    println!("axum serve returned unexpectedly");
    Ok(())
}

// fn run_scan_loop(
//     record_tx: broadcast::Sender<RecordMessage>,
//     hit_tx: broadcast::Sender<HitMessage>,
//     waterfall_tx: broadcast::Sender<WaterfallMessage>,
//     scanner_running: Arc<AtomicBool>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let source_id = "fm_sweep".to_string();
//     let sample_rate_hz = 2_048_000.0;
//     let dwell_ms = 20;
//     let fft_min_samples = 4096;
//
//     let planner_cfg = SweepCoverage {
//         start_hz: 88_000_000.0,
//         end_hz: 108_000_000.0,
//         sample_rate_hz,
//         usable_bandwidth_hz: 1_600_000.0,
//         overlap_fraction: 0.25,
//     };
//
//     let hotspots = [
//         (96_300_000.0, 2.0),
//         (99_500_000.0, 1.5),
//         (101_100_000.0, 1.5),
//     ];
//
//     let dev = open_first_rtlsdr()?;
//     let initial_planner = SweepPlanner::new_priority(planner_cfg.clone(), &hotspots);
//     let mut stream = RtlStream::open(dev, initial_planner.points()[0].center_hz, sample_rate_hz)?;
//
//     stream.activate()?;
//
//     let hit_cfg = HitDetectorConfig::default();
//
//     loop {
//         if !scanner_running.load(Ordering::Relaxed) {
//             std::thread::sleep(Duration::from_millis(100));
//             continue;
//         }
//
//         let mut planner = SweepPlanner::new_priority(planner_cfg.clone(), &hotspots);
//
//         while let Some(point) = planner.pop_next() {
//             if !scanner_running.load(Ordering::Relaxed) {
//                 break;
//             }
//
//
//
//             let samples = match dwell_capture(
//                 &mut stream,
//                 point.center_hz,
//                 dwell_ms,
//                 SettleStrategy::SleepAndFlush {
//                     millis: 5,
//                     flush_count: 2,
//                     timeout_us: 250_000,
//                 },
//             ) {
//                 Ok(samples) => samples,
//                 Err(err) => {
//                     let msg = err.to_string();
//
//                     if msg.contains("Overflow") {
//                         eprintln!("scanner overflow at {:.3} MHz; continuing", point.center_hz / 1_000_000.0);
//                         continue;
//                     }
//
//                     return Err(err);
//                 }
//             };
//
//             if samples.len() < fft_min_samples {
//                 continue;
//             }
//
//             let analysis = analyze(&samples[..fft_min_samples], point.center_hz, sample_rate_hz);
//
//             let timestamp_ms = now_ms();
//
//             let record = SweepRecord {
//                 center_hz: point.center_hz,
//                 lower_edge_hz: analysis.lower_edge_hz,
//                 upper_edge_hz: analysis.upper_edge_hz,
//                 avg_power: analysis.avg_power,
//                 noise_floor: analysis.noise_floor,
//                 peak_power: analysis.peak_power,
//                 peak_bin: analysis.peak_bin,
//                 estimated_peak_hz: analysis.estimated_peak_hz,
//                 timestamp_ms,
//             };
//
//             let record_msg = RecordMessage {
//                 kind: "record",
//                 source_id: source_id.clone(),
//                 timestamp_ms: record.timestamp_ms,
//                 center_hz: record.center_hz,
//                 lower_edge_hz: record.lower_edge_hz,
//                 upper_edge_hz: record.upper_edge_hz,
//                 avg_power: record.avg_power,
//                 noise_floor: record.noise_floor,
//                 peak_power: record.peak_power,
//                 peak_bin: record.peak_bin,
//                 estimated_peak_hz: record.estimated_peak_hz,
//             };
//
//             let _ = record_tx.send(record_msg);
//
//             let waterfall_msg = WaterfallMessage {
//                 kind: "waterfall_frame",
//                 source_id: source_id.clone(),
//                 timestamp_ms,
//                 center_hz: analysis.center_hz,
//                 lower_edge_hz: analysis.lower_edge_hz,
//                 upper_edge_hz: analysis.upper_edge_hz,
//                 bins: analysis.spectrum.clone(),
//             };
//
//             let _ = waterfall_tx.send(waterfall_msg);
//             println!("sent waterfall frame");
//
//             if let Some(hit) = detect_hit(
//                 &hit_cfg,
//                 &source_id,
//                 timestamp_ms,
//                 &analysis,
//                 fft_min_samples,
//             ) {
//                 log_hit(&hit);
//
//                 let hit_msg = HitMessage {
//                     kind: "hit",
//                     source_id: hit.source_id.clone(),
//                     timestamp_ms: hit.timestamp_ms,
//                     center_hz: hit.center_hz,
//                     peak_hz: hit.peak_hz,
//                     lower_edge_hz: hit.lower_edge_hz,
//                     upper_edge_hz: hit.upper_edge_hz,
//                     peak_bin: hit.peak_bin,
//                     peak_power: hit.peak_power,
//                     noise_floor: hit.noise_floor,
//                     avg_power: hit.avg_power,
//                     snr_db: hit.snr_db,
//                 };
//
//                 let _ = hit_tx.send(hit_msg);
//                 planner.reprioritize_near(hit.peak_hz, 0.75, 1_500_000.0);
//             }
//         }
//     }
// }

async fn scanner_status_handler(State(state): State<AppState>) -> Json<ScannerStatus> {
    Json(ScannerStatus {
        is_running: state.scanner_running.load(Ordering::Relaxed),
    })
}

async fn scanner_start_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.scanner_running.store(true, Ordering::Relaxed);
    (StatusCode::OK, Json(ScannerStatus { is_running: true }))
}

async fn scanner_stop_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.scanner_running.store(false, Ordering::Relaxed);
    (StatusCode::OK, Json(ScannerStatus { is_running: false }))
}

async fn ws_records_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| records_socket(socket, state))
}

async fn ws_hits_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| hits_socket(socket, state))
}

async fn ws_waterfall_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| waterfall_socket(socket, state))
}

async fn records_socket(socket: WebSocket, state: AppState) {
    println!("records websocket client connected");
    let mut rx = state.record_tx.subscribe();
    forward_broadcast_to_ws(socket, &mut rx).await;
    println!("records websocket client disconnected");
}

async fn hits_socket(socket: WebSocket, state: AppState) {
    println!("hits websocket client connected");
    let mut rx = state.hit_tx.subscribe();
    forward_broadcast_to_ws(socket, &mut rx).await;
    println!("hits websocket client disconnected");
}

async fn waterfall_socket(socket: WebSocket, state: AppState) {
    println!("waterfall websocket client connected");
    let mut rx = state.waterfall_tx.subscribe();
    forward_broadcast_to_ws(socket, &mut rx).await;
    println!("waterfall websocket client disconnected");
}

async fn forward_broadcast_to_ws<T>(
    mut socket: WebSocket,
    rx: &mut broadcast::Receiver<T>,
) where
    T: Serialize + Clone,
{
    loop {
        match rx.recv().await {
            Ok(msg) => {
                match serde_json::to_string(&msg) {
                    Ok(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("failed to serialize websocket message: {err}");
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                eprintln!("websocket client lagged, skipped {skipped} messages");
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn log_hit(hit: &Hit) {
    println!(
        "HIT center={:.3} MHz peak={:.3} MHz snr={:.2} dB peak={:.3} floor={:.3}",
        hit.center_hz / 1e6,
        hit.peak_hz / 1e6,
        hit.snr_db,
        hit.peak_power,
        hit.noise_floor,
    );
}