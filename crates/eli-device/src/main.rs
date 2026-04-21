use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;


use eli_device::scanner::args_vanilla::{DeviceKindArg, EdgeDeviceArgs};
use eli_device::edge_error::EdgeError;
use eli_device::scanner::args_vanilla::DeviceKindArg::BladeRf;
use eli_device::scanner::runner::ScannerRunner;
use eli_device::scanner::stream_device::rtl::RtlDevice;
use eli_device::scanner::stream_device::stream_vanilla::{DeviceStream, DeviceStreamWrapper};
use eli_protocol::edge_vanilla::scanner::config_vanilla::ScannerConfig;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::{EdgeCommand, EdgeEvent, StatusMessage};


#[tokio::main]
async fn main() -> Result<(), EdgeError> {
    let args = EdgeDeviceArgs::parse();

    let stream = UnixStream::connect(&args.socket_path).await?;
    let (read_half, mut write_half) = stream.into_split();

    let scanner_running = Arc::new(AtomicBool::new(false));
    let shutdown_requested = Arc::new(AtomicBool::new(false));
    let dropped_events = Arc::new(AtomicU64::new(0));

    let initial_config = ScannerConfig::default_for_worker(args.worker_id.clone());
    let pending_config = Arc::new(Mutex::new(Some(initial_config.clone())));
    let status_identity = Arc::new(Mutex::new((
        initial_config.edge_id.clone(),
        initial_config.source_id.clone(),
    )));


    let edge_device: Box<dyn DeviceStream> = match args.device_kind {
        DeviceKindArg::Rtl => {
            Box::new(RtlDevice::new(&args.serial_number)?)
        }
        _ => panic!("Unsupported device kind: {:?}", args.device_kind),
    };
    let wrapper = DeviceStreamWrapper(edge_device);
    
    let runner = ScannerRunner::new(
        wrapper,
        initial_config.clone(),
        pending_config.clone(),
        scanner_running.clone(),
        shutdown_requested.clone(),
        dropped_events.clone(),
    );

    let (event_tx, mut event_rx) = mpsc::channel::<EdgeEvent>(256);

    let _ = event_tx.try_send(EdgeEvent::Status(StatusMessage::new(
        initial_config.edge_id.clone(),
        initial_config.source_id.clone(),
        "connected",
        format!("worker {} connected to router", args.worker_id),
    )));

    let writer_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let line = serde_json::to_string(&event)?;
            write_half.write_all(line.as_bytes()).await?;
            write_half.write_all(b"\n").await?;
        }

        Ok::<(), EdgeError>(())
    });

    let scanner_running_for_cmd = scanner_running.clone();
    let shutdown_requested_for_cmd = shutdown_requested.clone();
    let pending_config_for_cmd = pending_config.clone();
    let dropped_events_for_cmd = dropped_events.clone();
    let event_tx_for_cmd = event_tx.clone();
    let status_identity_for_cmd = status_identity.clone();

    let command_task = tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();

        loop {
            match lines.next_line().await? {
                Some(line) => {
                    let cmd: EdgeCommand = serde_json::from_str(&line)?;

                    match cmd {
                        EdgeCommand::SetConfig(cfg) => {
                            let edge_id = cfg.edge_id.clone();
                            let source_id = cfg.source_id.clone();

                            {
                                let mut pending = pending_config_for_cmd.lock().unwrap();
                                *pending = Some(cfg);
                            }

                            {
                                let mut identity = status_identity_for_cmd.lock().unwrap();
                                *identity = (edge_id.clone(), source_id.clone());
                            }

                            let _ = event_tx_for_cmd.try_send(EdgeEvent::Status(
                                StatusMessage::new(
                                    edge_id,
                                    source_id,
                                    "config_pending",
                                    "received scanner configuration",
                                ),
                            ));
                        }

                        EdgeCommand::Start => {
                            scanner_running_for_cmd.store(true, Ordering::Relaxed);

                            let (edge_id, source_id) = {
                                let identity = status_identity_for_cmd.lock().unwrap();
                                identity.clone()
                            };

                            let _ = event_tx_for_cmd.try_send(EdgeEvent::Status(
                                StatusMessage::new(
                                    edge_id,
                                    source_id,
                                    "started",
                                    "scanner start requested",
                                ),
                            ));
                        }

                        EdgeCommand::Stop => {
                            scanner_running_for_cmd.store(false, Ordering::Relaxed);

                            let (edge_id, source_id) = {
                                let identity = status_identity_for_cmd.lock().unwrap();
                                identity.clone()
                            };

                            let _ = event_tx_for_cmd.try_send(EdgeEvent::Status(
                                StatusMessage::new(
                                    edge_id,
                                    source_id,
                                    "stopped",
                                    "scanner stop requested",
                                ),
                            ));
                        }

                        EdgeCommand::Ping => {
                            let dropped = dropped_events_for_cmd.load(Ordering::Relaxed);

                            let (edge_id, source_id) = {
                                let identity = status_identity_for_cmd.lock().unwrap();
                                identity.clone()
                            };

                            let _ = event_tx_for_cmd.try_send(EdgeEvent::Status(
                                StatusMessage::new(
                                    edge_id,
                                    source_id,
                                    "pong",
                                    format!("worker alive; dropped_events={dropped}"),
                                ),
                            ));
                        }

                        EdgeCommand::Shutdown => {
                            scanner_running_for_cmd.store(false, Ordering::Relaxed);
                            shutdown_requested_for_cmd.store(true, Ordering::Relaxed);

                            let (edge_id, source_id) = {
                                let identity = status_identity_for_cmd.lock().unwrap();
                                identity.clone()
                            };

                            let _ = event_tx_for_cmd.try_send(EdgeEvent::Status(
                                StatusMessage::new(
                                    edge_id,
                                    source_id,
                                    "shutdown",
                                    "worker shutdown requested",
                                ),
                            ));

                            break;
                        }
                    }
                }

                None => {
                    scanner_running_for_cmd.store(false, Ordering::Relaxed);
                    shutdown_requested_for_cmd.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }

        Ok::<(), EdgeError>(())
    });

    let scanner_task = tokio::task::spawn_blocking(move || runner.run_edge_loop(event_tx));

    let _ = tokio::try_join!(writer_task, command_task)?;
    scanner_task.await??;

    Ok(())
}