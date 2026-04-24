use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, RwLock};

use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;
use eli_protocol::router_vanilla::cmd_vanilla::{RouterCommand, RouterReply};
use eli_protocol::router_vanilla::result_vanilla::RouterResult;

use crate::router::runspec::config_helper::{fixed_config, fm_sweep_config, idle_config};
use crate::router::flux::state::RouterState;


pub fn remove_stale_socket(path: &Path) -> RouterResult<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub async fn handle_control_stream(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<RouterState>>,
) -> RouterResult<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    match reader.read_line(&mut line).await {
        Ok(0) => return Ok(()),
        Ok(_) => {}
        Err(err) => {
            eprintln!("[router] control read error: {err}");
            return Ok(());
        }
    }

    let reply = handle_router_command_line(&line, state).await;

    let json = serde_json::to_string(&reply)?;
    write_half.write_all(json.as_bytes()).await?;
    write_half.write_all(b"\n").await?;

    Ok(())
}

pub async fn handle_router_command_line(
    line: &str,
    state: Arc<Mutex<RouterState>>,
) -> RouterReply {
    match serde_json::from_str::<RouterCommand>(line) {
        Ok(command) => handle_router_command(command, state).await,
        Err(err) => RouterReply::Error {
            message: format!("invalid command: {err}"),
        },
    }
}

pub async fn handle_router_command(
    command: RouterCommand,
    state: Arc<Mutex<RouterState>>,
) -> RouterReply {
    match command {
        RouterCommand::Ping => RouterReply::Pong,

        RouterCommand::ListWorkers => {
            let state = state.lock().await;
            let worker_ids = state.workers.registry.keys().cloned().collect();

            RouterReply::Workers { worker_ids }
        }

        RouterCommand::StopWorker { worker_id } => {
            send_worker_event(state, worker_id, EdgeEvent::Stop, "stop").await
        }

        RouterCommand::StartWorker { worker_id } => {
            send_worker_event(state, worker_id, EdgeEvent::Start, "start").await
        }

        RouterCommand::SetIdle { worker_id } => {
            let cfg = idle_config(&worker_id);
            send_worker_event(state, worker_id, EdgeEvent::SetConfig(cfg), "idle config").await
        }

        RouterCommand::SetSweepFm { worker_id } => {
            let cfg = fm_sweep_config(&worker_id);
            send_worker_event(state, worker_id, EdgeEvent::SetConfig(cfg), "fm sweep config").await
        }

        RouterCommand::SetFixed {
            worker_id,
            center_hz,
        } => {
            let cfg = fixed_config(&worker_id, center_hz);
            send_worker_event(
                state,
                worker_id,
                EdgeEvent::SetConfig(cfg),
                &format!("fixed config ({:.3} MHz)", center_hz / 1e6),
            )
                .await
        }
    }
}

pub async fn send_worker_event(
    state: Arc<Mutex<RouterState>>,
    worker_id: String,
    event: EdgeEvent,
    label: &str,
) -> RouterReply {
    let tx = {
        let state = state.lock().await;
        state.workers.get_command_sender(&worker_id)
    };

    match tx {
        Some(tx) => match tx.send(event).await {
            Ok(_) => RouterReply::Ok {
                message: format!("{label} sent to {worker_id}"),
            },
            Err(err) => RouterReply::Error {
                message: format!("failed sending {label} to {worker_id}: {err}"),
            },
        },
        None => RouterReply::Error {
            message: format!("unknown worker {worker_id}"),
        },
    }
}