use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use eli_edge_device::capture::discovery::open_rtlsdr_by_index;
use eli_edge_device::capture::stream::RtlStream;
use eli_edge_device::scanner::config::ScannerConfig;
use eli_edge_device::scanner::runner::ScannerRunner;
use eli_edge_device::scanner::vanilla::EdgeEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
struct EdgeDeviceArgs {
    #[arg(long)]
    worker_id: String,

    #[arg(long)]
    socket_path: String,

    #[arg(long)]
    device_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeCommand {
    SetConfig(ScannerConfig),
    Start,
    Stop,
    Ping,
    Shutdown,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = EdgeDeviceArgs::parse();

    let stream = UnixStream::connect(&args.socket_path).await?;
    let (read_half, mut write_half) = stream.into_split();

    let scanner_running = Arc::new(AtomicBool::new(false));

    let dev = open_rtlsdr_by_index(args.device_index)?;
    let initial_center_hz = 96_300_000.0;
    let initial_sample_rate_hz = 2_048_000.0;
    let rtl_stream = RtlStream::open(dev, initial_center_hz, initial_sample_rate_hz)?;

    let initial_config = ScannerConfig::default_for_worker(args.worker_id.clone());

    let runner = Arc::new(Mutex::new(ScannerRunner::new(
        rtl_stream,
        initial_config,
        scanner_running.clone(),
    )));

    let (event_tx, mut event_rx) = mpsc::channel::<EdgeEvent>(256);

    // writer task: send edge events to router
    let writer_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let line = serde_json::to_string(&event)?;
            write_half.write_all(line.as_bytes()).await?;
            write_half.write_all(b"\n").await?;
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // command reader task
    let runner_for_cmd = runner.clone();
    let scanner_running_for_cmd = scanner_running.clone();
    let command_task = tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();

        while let Some(line) = lines.next_line().await? {
            let cmd: EdgeCommand = serde_json::from_str(&line)?;

            match cmd {
                EdgeCommand::SetConfig(cfg) => {
                    let mut runner = runner_for_cmd.lock().unwrap();
                    runner.pending_config = Some(cfg);
                }
                EdgeCommand::Start => {
                    scanner_running_for_cmd.store(true, Ordering::Relaxed);
                }
                EdgeCommand::Stop => {
                    scanner_running_for_cmd.store(false, Ordering::Relaxed);
                }
                EdgeCommand::Ping => {
                    // later: emit a Status/Pong event
                }
                EdgeCommand::Shutdown => {
                    scanner_running_for_cmd.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // scanner task
    let runner_for_scan = runner.clone();
    let scanner_task = tokio::task::spawn_blocking(move || {
        let mut runner = runner_for_scan.lock().unwrap();
        runner.run_edge_loop(event_tx);
    });

    let _ = tokio::try_join!(writer_task, command_task)?;
    let _ = scanner_task.await?;

    Ok(())
}