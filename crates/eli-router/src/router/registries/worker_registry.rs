use tokio::process::{Child, Command};
use crate::{RouterError, RouterResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeCommand;
use crate::router::flux::state::RouterState;

pub struct WorkerHandle {
    pub worker_id: String,
    pub device_id: String,
    pub command_tx: mpsc::Sender<EdgeCommand>,
    pub connected: bool,
    pub socket_path: String,
    pub child: Option<Child>,
}

pub struct WorkerRegistry {
    workers: HashMap<String, WorkerHandle>,
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    pub fn register(&mut self, worker: WorkerHandle) {
        self.workers.insert(worker.worker_id.clone(), worker);
    }

    pub fn get(&self, id: &str) -> Option<&WorkerHandle> {
        self.workers.get(id)
    }

    pub async fn spawn_edge_worker(
        &mut self,
        edge_device_bin: &str,
        worker_id: &str,
        socket_path: &str,
        device_index: usize,
    ) -> RouterResult<()> {
        let child = Command::new(edge_device_bin)
            .arg("--worker-id")
            .arg(worker_id)
            .arg("--socket-path")
            .arg(socket_path)
            .arg("--device-index")
            .arg(device_index.to_string())
            .spawn()?;

        self.workers.insert(worker_id.to_string(), WorkerHandle {
            worker_id: worker_id.to_string(),
            device_id: format!("device_{}", device_index),
            command_tx: mpsc::channel(100).0,
            connected: true,
            socket_path: socket_path.to_string(),
            child: Some(child),
        });

        Ok(())
    }
    async fn handle_worker_accept(
        worker_id: String,
        listener: tokio::net::UnixListener,
        state: Arc<tokio::sync::Mutex<RouterState>>,
    ) -> Result<(), RouterError> {
        let (stream, _) = listener.accept().await?;
        let (read_half, mut write_half) = stream.into_split();

        let (command_tx, mut command_rx) = tokio::sync::mpsc::channel(256);

        {
            let mut state = state.lock().await;
            if let Some(worker) = state.workers.get_mut(&worker_id) {
                worker.command_tx = command_tx.clone();
                worker.connected = true;
            }
        }

        let write_task = tokio::spawn(async move {
            while let Some(cmd) = command_rx.recv().await {
                let line = serde_json::to_string(&cmd)?;
                use tokio::io::AsyncWriteExt;
                write_half.write_all(line.as_bytes()).await?;
                write_half.write_all(b"\n").await?;
            }
            Ok::<(), RouterError>(())
        });

        let state_for_read = state.clone();
        let read_task = tokio::spawn(async move {
            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut lines = BufReader::new(read_half).lines();

            while let Some(line) = lines.next_line().await? {
                let event: eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent =
                    serde_json::from_str(&line)?;

                let router_event = crate::router::vanilla::RouterEvent {
                    worker_id: worker_id.clone(),
                    source_id: extract_source_id(&event),
                    timestamp_ms: extract_timestamp_ms(&event),
                    event,
                };

                let state = state_for_read.lock().await;
                state.listeners.publish(router_event);
            }

            Ok::<(), RouterError>(())
        });

        let _ = tokio::try_join!(write_task, read_task)?;
        Ok(())
    }
}