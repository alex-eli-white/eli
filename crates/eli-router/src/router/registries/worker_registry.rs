use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use eli_protocol::edge_vanilla::scanner::msg_vanilla::{EdgeCommand, EdgeEvent};

use crate::router::registries::reg_vanilla::{DeviceBackend, DeviceIdentity};
use crate::router::vanilla::RouterEvent;
use crate::{RouterError, RouterResult};

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
    pub command_tx: mpsc::Sender<EdgeCommand>,
    pub state: WorkerState,
    pub last_event_timestamp_ms: Option<u128>,
    pub child: Child,
}

pub struct WorkerRegistry {
    workers: HashMap<String, WorkerHandle>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    pub fn contains_device(&self, device: &DeviceIdentity) -> bool {
        self.workers.values().any(|worker| &worker.device == device)
    }

    pub fn contains_worker_id(&self, worker_id: &str) -> bool {
        self.workers.contains_key(worker_id)
    }

    pub fn running_worker_ids(&self) -> HashSet<String> {
        self.workers.keys().cloned().collect()
    }

    pub async fn prune_exited(&mut self) -> RouterResult<Vec<String>> {
        let ids: Vec<String> = self.workers.keys().cloned().collect();
        let mut exited = Vec::new();

        for worker_id in ids {
            let should_remove = if let Some(worker) = self.workers.get_mut(&worker_id) {
                match worker.child.try_wait()? {
                    Some(status) => {
                        println!(
                            "[router] worker={} exited status={}",
                            worker.worker_id, status
                        );
                        true
                    }
                    None => false,
                }
            } else {
                false
            };

            if should_remove {
                self.workers.remove(&worker_id);
                exited.push(worker_id);
            }
        }

        Ok(exited)
    }

    pub fn update_worker_running(&mut self, worker_id: &str) {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.state = WorkerState::Running;
        }
    }

    pub fn update_last_event_timestamp(&mut self, worker_id: &str, timestamp_ms: u128) {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.last_event_timestamp_ms = Some(timestamp_ms);
        }
    }

    pub async fn send_command(
        &self,
        worker_id: &str,
        cmd: EdgeCommand,
    ) -> RouterResult<()> {
        let worker = self
            .workers
            .get(worker_id)
            .ok_or_else(|| RouterError::Message(format!("unknown worker: {worker_id}")))?;

        worker
            .command_tx
            .send(cmd)
            .await
            .map_err(|_| RouterError::Message(format!("failed to send command to worker: {worker_id}")))
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

        println!("[router] cwd: {:?}", std::env::current_dir()?);
        println!(
            "[router] edge binary exists? {}",
            edge_device_bin.exists()
        );
        println!(
            "[router] edge binary canonical: {:?}",
            std::fs::canonicalize(edge_device_bin)
        );
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

        println!("[router] spawning command: {:?}", cmd);

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

        self.workers.insert(worker_id.clone(), handle);

        tokio::spawn(async move {
            if let Err(err) = handle_worker_connection(listener, worker_id.clone(), event_tx, command_rx).await {
                eprintln!("[router] worker={} connection task failed: {}", worker_id, err);
            }
        });

        Ok(())
    }
}
async fn handle_worker_connection(
    listener: UnixListener,
    worker_id: String,
    event_tx: mpsc::Sender<RouterEvent>,
    mut command_rx: mpsc::Receiver<EdgeCommand>,
) -> RouterResult<()> {
    let (stream, _) = listener.accept().await?;
    let (read_half, mut write_half) = stream.into_split();

    let writer = tokio::spawn(async move {
        while let Some(cmd) = command_rx.recv().await {
            let line = serde_json::to_string(&cmd)?;
            write_half.write_all(line.as_bytes()).await?;
            write_half.write_all(b"\n").await?;
        }
        Ok::<(), RouterError>(())
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
        EdgeEvent::Status(msg) => msg.edge_id.clone(),
        EdgeEvent::Record(msg) => msg.record_ctx.source_id.clone(),
        EdgeEvent::Waterfall(msg) => msg.record_ctx.source_id.clone(),
        EdgeEvent::IqChunk(msg) => msg.record_ctx.source_id.clone(),
    }
}

fn extract_timestamp_ms(event: &EdgeEvent) -> u128 {
    match event {
        EdgeEvent::Status(msg) => msg.timestamp_ms,
        EdgeEvent::Record(msg) => msg.record_ctx.timestamp_ms,
        EdgeEvent::Waterfall(msg) => msg.record_ctx.timestamp_ms,
        EdgeEvent::IqChunk(msg) => msg.record_ctx.timestamp_ms,
    }
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
