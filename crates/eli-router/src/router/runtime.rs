use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use soapysdr_sys::SoapySDRRange;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, mpsc, Mutex};
use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;
use eli_protocol::router_vanilla::cmd_vanilla::{RouterCommand, RouterEvent, RouterReply};
use eli_protocol::router_vanilla::device_vanilla::{ControlLease, DeviceDescriptor, DeviceDiscovery, DeviceIdentity};
use eli_protocol::router_vanilla::result_vanilla::{RouterError, RouterResult};
use crate::router::config_helper::{fixed_config, fm_sweep_config, idle_config};
use crate::router::flux::state::RouterState;
use crate::router::flux::event_fanout::new_router_broadcast;
use crate::router::genesis::rtl_genesis::RtlSdrDiscovery;

use crate::router::registries::worker_registry::now_ms;


pub struct RouterRuntime {
    pub socket_dir: PathBuf,
    pub edge_device_bin: PathBuf,
    pub discovery_interval: Duration,
    pub state: Arc<Mutex<RouterState>>,
    pub ingress_tx: mpsc::Sender<RouterEvent>,
    ingress_rx: Option<mpsc::Receiver<RouterEvent>>,
}

impl RouterRuntime {
    pub fn new(
        socket_dir: PathBuf,
        edge_device_bin: PathBuf,
        discovery_interval_secs: u64,
    ) -> Self {
        let broadcaster = new_router_broadcast(1024);
        let state = Arc::new(Mutex::new(RouterState::new(broadcaster)));
        let (ingress_tx, ingress_rx) = mpsc::channel(1024);

        Self {
            socket_dir,
            edge_device_bin,
            discovery_interval: Duration::from_secs(discovery_interval_secs),
            state,
            ingress_tx,
            ingress_rx: Some(ingress_rx),
        }
    }

    pub async fn run(&mut self) -> RouterResult<()> {
        self.ensure_socket_dir().await?;
        self.spawn_event_ingress_task()?;
        self.spawn_control_listener()?;
        self.spawn_debug_observer().await;

        self.reconcile_once().await?;

        loop {
            tokio::time::sleep(self.discovery_interval).await;
        }
    }
    async fn ensure_socket_dir(&self) -> RouterResult<()> {
        tokio::fs::create_dir_all(&self.socket_dir).await?;



        Ok(())
    }

    fn control_socket_path(&self) -> PathBuf {
        self.socket_dir.join("router-control.sock")
    }

    fn remove_stale_socket(path: &Path) -> RouterResult<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    fn spawn_control_listener(&self) -> RouterResult<()> {
        let socket_path = self.control_socket_path();

        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Self::remove_stale_socket(&socket_path)?;

        let listener = UnixListener::bind(&socket_path)?;
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(parts) => parts,
                    Err(err) => {
                        eprintln!("[router] control accept error: {}", err);
                        continue;
                    }
                };

                let state = Arc::clone(&state);

                tokio::spawn(async move {
                    let (read_half, mut write_half) = stream.into_split();
                    let mut reader = BufReader::new(read_half);
                    let mut line = String::new();

                    match reader.read_line(&mut line).await {
                        Ok(0) => return,
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("[router] control read error: {}", err);
                            return;
                        }
                    }

                    let reply = match serde_json::from_str::<RouterCommand>(&line) {
                        Ok(RouterCommand::Ping) => RouterReply::Pong,

                        Ok(RouterCommand::ListWorkers) => {
                            let state = state.lock().await;
                            let worker_ids = state.workers.registry.keys().cloned().collect();
                            RouterReply::Workers { worker_ids }
                        }

                        Ok(RouterCommand::StopWorker { worker_id }) => {
                            let tx = {
                                let state = state.lock().await;
                                state.workers.get_command_sender(&worker_id)
                            };

                            match tx {
                                Some(tx) => match tx.send(EdgeEvent::Stop).await {
                                    Ok(_) => RouterReply::Ok {
                                        message: format!("stop sent to {}", worker_id),
                                    },
                                    Err(err) => RouterReply::Error {
                                        message: format!("failed sending stop to {}: {}", worker_id, err),
                                    },
                                },
                                None => RouterReply::Error {
                                    message: format!("unknown worker {}", worker_id),
                                },
                            }
                        }

                        Ok(RouterCommand::StartWorker { worker_id }) => {
                            let tx = {
                                let state = state.lock().await;
                                state.workers.get_command_sender(&worker_id)
                            };



                            match tx {
                                Some(tx) => match tx.send(EdgeEvent::Start).await {
                                    Ok(_) => RouterReply::Ok {
                                        message: format!("start sent to {}", worker_id),
                                    },
                                    Err(err) => RouterReply::Error {
                                        message: format!("failed sending start to {}: {}", worker_id, err),
                                    },
                                },
                                None => RouterReply::Error {
                                    message: format!("unknown worker {}", worker_id),
                                },
                            }
                        }

                        Ok(RouterCommand::SetIdle { worker_id }) => {
                            let tx = {
                                let state = state.lock().await;
                                state.workers.get_command_sender(&worker_id)
                            };

                            match tx {
                                Some(tx) => {
                                    let cfg = idle_config(&worker_id);
                                    match tx.send(EdgeEvent::SetConfig(cfg)).await {
                                        Ok(_) => RouterReply::Ok {
                                            message: format!("idle config sent to {}", worker_id),
                                        },
                                        Err(err) => RouterReply::Error {
                                            message: format!("failed sending idle config to {}: {}", worker_id, err),
                                        },
                                    }
                                }
                                None => RouterReply::Error {
                                    message: format!("unknown worker {}", worker_id),
                                },
                            }
                        }

                        Ok(RouterCommand::SetSweepFm { worker_id }) => {
                            let tx = {
                                let state = state.lock().await;
                                state.workers.get_command_sender(&worker_id)
                            };


                            match tx {
                                Some(tx) => {
                                    let cfg = fm_sweep_config(&worker_id);
                                    match tx.send(EdgeEvent::SetConfig(cfg)).await {
                                        Ok(_) => RouterReply::Ok {
                                            message: format!("fm sweep config sent to {}", worker_id),
                                        },
                                        Err(err) => RouterReply::Error {
                                            message: format!("failed sending fm sweep config to {}: {}", worker_id, err),
                                        },
                                    }
                                }
                                None => RouterReply::Error {
                                    message: format!("unknown worker {}", worker_id),
                                },
                            }
                        }

                        Ok(RouterCommand::SetFixed { worker_id, center_hz }) => {
                            let tx = {
                                let state = state.lock().await;
                                state.workers.get_command_sender(&worker_id)
                            };

                            match tx {
                                Some(tx) => {
                                    let cfg = fixed_config(&worker_id, center_hz);
                                    match tx.send(EdgeEvent::SetConfig(cfg)).await {
                                        Ok(_) => RouterReply::Ok {
                                            message: format!("fixed config sent to {} ({:.3} MHz)", worker_id, center_hz / 1e6),
                                        },
                                        Err(err) => RouterReply::Error {
                                            message: format!("failed sending fixed config to {}: {}", worker_id, err),
                                        },
                                    }
                                }
                                None => RouterReply::Error {
                                    message: format!("unknown worker {}", worker_id),
                                },
                            }
                        }

                        Err(err) => RouterReply::Error {
                            message: format!("invalid command: {}", err),
                        },
                    };

                    match serde_json::to_string(&reply) {
                        Ok(json) => {
                            if let Err(err) = write_half.write_all(json.as_bytes()).await {
                                return;
                            }
                            if let Err(err) = write_half.write_all(b"\n").await {
                                eprintln!("[router] control write newline error: {}", err);
                            }
                        }
                        Err(err) => {
                            eprintln!("[router] control serialize error: {}", err);
                        }
                    }
                });
            }
        });

        Ok(())
    }

    fn spawn_event_ingress_task(&mut self) -> RouterResult<()> {
        let mut rx = self
            .ingress_rx
            .take()
            .ok_or_else(|| RouterError::Message("ingress task already started".to_string()))?;
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut state_guard = state.lock().await;
                state_guard.workers.update_worker_running(&event.worker_id);
                state_guard
                    .workers
                    .update_last_event_timestamp(&event.worker_id, event.timestamp_ms);
                let _ = state_guard.broadcaster.send(event);
            }
        });

        Ok(())
    }

    async fn spawn_debug_observer(&self) {
        let tx = {
            let state = self.state.lock().await;
            state.broadcaster.clone()
        };

        let mut rx = tx.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {

                        match &event.event {
                            EdgeEvent::Status(msg) => {
                                // println!(
                                //     "[router] worker={} source={} STATUS status={} message={}",
                                //     event.worker_id,
                                //     event.source_id,
                                //     msg.status,
                                //     msg.message,
                                // );
                            }
                            other => {
                                println!(
                                    "[router] worker={} source={} EVENT {:?}",
                                    event.worker_id,
                                    event.source_id,
                                    other,
                                );
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        eprintln!("[router] debug observer lagged and dropped {} events", count);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    pub async fn reconcile_once(&self) -> RouterResult<()> {
        let discovered = self.discover_devices()?;

        {
            let mut state = self.state.lock().await;
            let _ = state.workers.prune_exited().await?;
        }

        for descriptor in discovered {
            let Some(device) = self.descriptor_to_identity(&descriptor) else {
                continue;
            };

            let already_running = {
                let state = self.state.lock().await;
                state.workers.contains_device(&device)
            };

            if already_running {
                continue;
            }

            let mut state = self.state.lock().await;

            state
                .workers
                .spawn_edge_worker(
                    &self.edge_device_bin,
                    &self.socket_dir,
                    device,
                    self.ingress_tx.clone(),
                )
                .await?;
        }

        Ok(())
    }

    pub fn discover_devices(&self) -> RouterResult<Vec<DeviceDescriptor<Vec<SoapySDRRange>>>> {
        let rtl = RtlSdrDiscovery;
        rtl.discover()
    }

    fn descriptor_to_identity(
        &self,
        descriptor: &DeviceDescriptor<Vec<SoapySDRRange>>,
    ) -> Option<DeviceIdentity> {
        let serial_number = descriptor.serial_number.clone()?;
        Some(DeviceIdentity {
            backend: descriptor.backend.clone(),
            serial_number,
        })
    }

    pub async fn try_claim_control(&self, controller_id: impl Into<String>) -> bool {
        let controller_id = controller_id.into();
        let mut state = self.state.lock().await;

        if state.control_lease.is_some() {
            return false;
        }

        state.control_lease = Some(ControlLease {
            controller_id,
            issued_at_ms: now_ms(),
        });

        true
    }

    pub async fn release_control(&self, controller_id: &str) {
        let mut state = self.state.lock().await;
        if let Some(lease) = &state.control_lease && lease.controller_id == controller_id {
                state.control_lease = None
        }
    }
}