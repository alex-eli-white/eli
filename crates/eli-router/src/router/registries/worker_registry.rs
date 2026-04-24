use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use eli_device::helpers::writer_helper::writer_task_helper;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::{EdgeEvent};
use eli_protocol::router_vanilla::cmd_vanilla::RouterEvent;
use eli_protocol::router_vanilla::device_vanilla::DeviceIdentity;
use eli_protocol::router_vanilla::result_vanilla::{RouterError, RouterResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Starting,
    Running,
    Exited,
}

pub struct WorkerHandle {
    pub worker_id: String,
    pub device: DeviceIdentity,
    pub socket_path: String,
    pub command_tx: mpsc::Sender<EdgeEvent>,
    pub state: WorkerState,
    pub last_event_timestamp_ms: Option<u128>,
    pub child: Child,
}

pub struct WorkerRegistry {
    pub(crate) registry: HashMap<String, WorkerHandle>,
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn contains_device(&self, device: &DeviceIdentity) -> bool {
        self.registry.values().any(|worker| &worker.device == device)
    }

    pub fn contains_worker_id(&self, worker_id: &str) -> bool {
        self.registry.contains_key(worker_id)
    }

    pub fn worker_ids(&self) -> Vec<String> {
        self.registry.keys().cloned().collect()
    }

    pub fn get_command_sender(&self, worker_id: &str) -> Option<mpsc::Sender<EdgeEvent>> {
        self.registry.get(worker_id).map(|h| h.command_tx.clone())
    }

    pub async fn prune_exited(&mut self) -> RouterResult<Vec<String>> {
        let ids: Vec<String> = self.registry.keys().cloned().collect();
        let mut exited = Vec::new();

        for worker_id in ids {
            let should_remove = if let Some(worker) = self.registry.get_mut(&worker_id) {
                match worker.child.try_wait()? {
                    Some(status) => {
                        println!(
                            "[router] worker={} exited status={}",
                            worker.worker_id, status
                        );
                        worker.state = WorkerState::Exited;
                        true
                    }
                    None => false,
                }
            } else {
                false
            };

            if should_remove {
                if let Some(worker) = self.registry.remove(&worker_id) {
                    let socket_path = PathBuf::from(&worker.socket_path);
                    if socket_path.exists() {
                        let _ = std::fs::remove_file(&socket_path);
                    }
                }
                exited.push(worker_id);
            }
        }

        Ok(exited)
    }

    pub fn update_worker_running(&mut self, worker_id: &str) {
        if let Some(worker) = self.registry.get_mut(worker_id) {
            worker.state = WorkerState::Running;
        }
    }

    pub fn update_last_event_timestamp(&mut self, worker_id: &str, timestamp_ms: u128) {
        if let Some(worker) = self.registry.get_mut(worker_id) {
            worker.last_event_timestamp_ms = Some(timestamp_ms);
        }
    }

    pub async fn send_command(
        &self,
        worker_id: &str,
        cmd: EdgeEvent,
    ) -> RouterResult<()> {
        let worker = self
            .registry
            .get(worker_id)
            .ok_or_else(|| RouterError::Message(format!("unknown worker: {worker_id}")))?;

        worker.command_tx.send(cmd).await.map_err(|_| {
            RouterError::Message(format!(
                "failed to send command to worker: {worker_id}"
            ))
        })
    }

    pub async fn spawn_edge_worker(
        &mut self,
        edge_device_bin: &Path,
        socket_dir: &Path,
        device: DeviceIdentity,
        event_tx: mpsc::Sender<RouterEvent>,
    ) -> RouterResult<()> {
        let worker_id = device.worker_id();
        if self.contains_worker_id(&worker_id) {
            return Ok(());
        }

        let socket_path = socket_dir.join(device.socket_name());

        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if socket_path.exists() {
            let _ = std::fs::remove_file(&socket_path);
        }


        let listener = UnixListener::bind(&socket_path)?;
        let socket_path_str = socket_path.to_string_lossy().to_string();

        let mut cmd = Command::new(edge_device_bin);
        cmd.arg("--worker-id")
            .arg(&worker_id)
            .arg("--socket-path")
            .arg(&socket_path_str)
            .arg("--device-index")
            .arg("0")
            .arg("--device-kind")
            .arg(device.backend.cli_value())
            .arg("--serial-number")
            .arg(&device.serial_number)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        
        let child = cmd.spawn()?;

        let (command_tx, command_rx) = mpsc::channel(256);

        let handle = WorkerHandle {
            worker_id: worker_id.clone(),
            device: device.clone(),
            socket_path: socket_path_str.clone(),
            command_tx,
            state: WorkerState::Starting,
            last_event_timestamp_ms: None,
            child,
        };

        self.registry.insert(worker_id.clone(), handle);

        tokio::spawn(async move {
            if let Err(err) =
                handle_worker_connection(listener, worker_id.clone(), event_tx, command_rx).await
            {
                eprintln!(
                    "[router] worker={} connection task failed: {}",
                    worker_id, err
                );
            }
        });

        Ok(())
    }
}

async fn handle_worker_connection(
    listener: UnixListener,
    worker_id: String,
    event_tx: mpsc::Sender<RouterEvent>,
    command_rx: mpsc::Receiver<EdgeEvent>,
) -> RouterResult<()> {
    let (stream, _) = listener.accept().await?;
    let (read_half, write_half) = stream.into_split();

    let writer = tokio::spawn(async move {
        writer_task_helper(write_half, command_rx).await
    });

    let reader = tokio::spawn(async move {
        let mut lines = BufReader::new(read_half).lines();

        while let Some(line) = lines.next_line().await? {
            let event: EdgeEvent = serde_json::from_str(&line)?;
            let router_event = RouterEvent {
                worker_id: worker_id.clone(),
                source_id: extract_source_id(&event),
                timestamp_ms: extract_timestamp_ms(&event),
                event,
            };

            if event_tx.send(router_event).await.is_err() {
                return Ok::<(), RouterError>(());
            }
        }

        Ok::<(), RouterError>(())
    });

    let _ = tokio::try_join!(writer, reader)?;
    Ok(())
}

fn extract_source_id(event: &EdgeEvent) -> String {
    match event {
        EdgeEvent::Status(msg) => msg.source_id.clone(),
        EdgeEvent::Record(msg) => msg.record_ctx.source_id.clone(),
        EdgeEvent::Waterfall(msg) => msg.record_ctx.source_id.clone(),
        EdgeEvent::IqChunk(msg) => msg.record_ctx.source_id.clone(),
        EdgeEvent::Hello(msg) => msg.source_id.clone(),
        _ => "source-not-available".to_string(),
    }
}

fn extract_timestamp_ms(event: &EdgeEvent) -> u128 {
    match event {
        EdgeEvent::Status(msg) => msg.timestamp_ms,
        EdgeEvent::Record(msg) => msg.record_ctx.timestamp_ms,
        EdgeEvent::Waterfall(msg) => msg.record_ctx.timestamp_ms,
        EdgeEvent::IqChunk(msg) => msg.record_ctx.timestamp_ms,
        EdgeEvent::Hello(msg) => msg.timestamp_ms,
        _ => now_ms(),
    }
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}