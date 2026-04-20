use std::path::Path;

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::time::{sleep, Duration};
use eli_edge_router::router_error::RouterError;
use eli_edge_router::RouterResult;
use eli_protocol::edge_vanilla::scanner::config_vanilla::{FixedModeConfig, ScannerConfig, ScannerMode};
use eli_protocol::edge_vanilla::scanner::dwell_vanilla::SettleStrategy;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::{EdgeCommand, EdgeEvent, IqCaptureMode};

#[derive(Debug, Parser)]
struct RouterArgs {
    #[arg(long)]
    worker_id: String,

    #[arg(long)]
    socket_path: String,

    #[arg(long, default_value_t = false)]
    auto_start_fixed: bool,
}

#[tokio::main]
async fn main() -> Result<(), RouterError> {
    let args = RouterArgs::parse();

    if let Some(parent) = Path::new(&args.socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    if Path::new(&args.socket_path).exists() {
        tokio::fs::remove_file(&args.socket_path).await?;
    }

    let listener = UnixListener::bind(&args.socket_path)?;
    println!("router listening for worker {} on {}", args.worker_id, args.socket_path);

    let (stream, _) = listener.accept().await?;
    println!("worker connected");

    let (read_half, mut write_half) = stream.into_split();

    let reader_task: tokio::task::JoinHandle<RouterResult<()>> = tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();
        while let Some(line) = lines.next_line().await? {
            let event: EdgeEvent = serde_json::from_str(&line)?;
            println!("event: {:?}", event);
        }
        Ok(())
    });

    write_command(&mut write_half, &EdgeCommand::Ping).await?;

    if args.auto_start_fixed {
        let cfg = demo_fixed_config(args.worker_id.clone());
        write_command(&mut write_half, &EdgeCommand::SetConfig(cfg)).await?;
        write_command(&mut write_half, &EdgeCommand::Start).await?;
    }

    sleep(Duration::from_secs(3)).await;
    write_command(&mut write_half, &EdgeCommand::Ping).await?;

    reader_task.await??;
    Ok(())
}

async fn write_command(
    write_half: &mut tokio::net::unix::OwnedWriteHalf,
    cmd: &EdgeCommand,
) -> RouterResult<()> {
    let line = serde_json::to_string(cmd)?;
    write_half.write_all(line.as_bytes()).await?;
    write_half.write_all(b"\n").await?;
    Ok(())
}

fn demo_fixed_config(worker_id: String) -> ScannerConfig {
    ScannerConfig {
        source_id: "rtl-sdr-0".to_string(),
        edge_id: worker_id,
        sample_rate_hz: 2_048_000.0,
        settle: SettleStrategy::SleepAndFlush {
            millis: 5,
            flush_count: 2,
            timeout_us: 250_000,
        },
        mode: ScannerMode::Fixed(FixedModeConfig {
            center_hz: 96_300_000.0,
            sample_rate_hz: 2_048_000.0,
            dwell_ms: 250,
            fft_min_samples: 4096,
            settle: SettleStrategy::SleepAndFlush {
                millis: 5,
                flush_count: 2,
                timeout_us: 250_000,
            },
            iq_capture: IqCaptureMode::Off,
            iq_chunk_samples: 4096,
        }),
    }
}
