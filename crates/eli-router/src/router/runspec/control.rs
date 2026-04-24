use std::path::{PathBuf};
use std::sync::{Arc};

use tokio::net::UnixListener;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{broadcast, Mutex};
use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;
use eli_protocol::router_vanilla::cmd_vanilla::{RouterEvent};
use eli_protocol::router_vanilla::result_vanilla::{RouterError, RouterResult};

use crate::router::flux::state::RouterState;
use crate::router::runspec::io_handler::{handle_control_stream, remove_stale_socket};
use crate::router::runspec::devctl::DeviceCtl;

pub struct IoCtl {
    pub socket_dir: PathBuf,
    pub ctl_rx: Option<Receiver<RouterEvent>>,
    pub ctl_tx: Option<Sender<RouterEvent>>,
    pub state: Arc<Mutex<RouterState>>,
}


impl IoCtl {


    pub fn new(socket_dir: PathBuf, state: Arc<Mutex<RouterState>>,ctl_rx: Option<Receiver<RouterEvent>>, ctl_tx: Option<Sender<RouterEvent>>) -> Self {
        Self { socket_dir, state, ctl_rx, ctl_tx }
    }

    pub fn control_socket_path(&self) -> PathBuf {
        self.socket_dir.join("router-control.sock")
    }

    pub fn spawn_event_ingress_task(&mut self) -> RouterResult<()> {
        let mut rx = self
            .ctl_rx
            .take()
            .ok_or_else(|| RouterError::Message("ingress task already started".to_string()))?;

        let state = Arc::clone(&self.state);


        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let mut state = state.lock().await;

                state.workers.update_worker_running(&event.worker_id);
                state
                    .workers
                    .update_last_event_timestamp(&event.worker_id, event.timestamp_ms);

                let _ = state.broadcaster.send(event);
            }
        });

        Ok(())
    }

    pub(crate) async fn ensure_socket_dir(&self) -> RouterResult<()> {
        tokio::fs::create_dir_all(&self.socket_dir).await?;
        Ok(())
    }

    pub(crate) fn spawn_control_listener(&self) -> RouterResult<()> {
        let socket_path = self.control_socket_path();

        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        remove_stale_socket(&socket_path)?;

        let listener = UnixListener::bind(&socket_path)?;
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(parts) => parts,
                    Err(err) => {
                        eprintln!("[router] control accept error: {err}");
                        continue;
                    }
                };

                let state = Arc::clone(&state);

                tokio::spawn(async move {
                    let reply = handle_control_stream(stream, state).await;

                    if let Err(err) = reply {
                        eprintln!("[router] control stream error: {err}");
                    }
                });
            }
        });

        Ok(())
    }

    pub async fn spawn_debug_observer(&self) {
        let tx = {
            let state = self.state.lock().await;
            state.broadcaster.clone()
        };

        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => match &event.event {
                        EdgeEvent::Status(_) => {}
                        other => {
                            println!(
                                "[router] worker={} source={} EVENT {:?}",
                                event.worker_id, event.source_id, other
                            );
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        eprintln!("[router] debug observer lagged and dropped {count} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }


    pub async fn spawn_from_device(&mut self) -> RouterResult<()> {
        let discovered = DeviceCtl::discover_devices()?;

        {
            let mut state = self.state.lock().await;
            let _ = state.workers.prune_exited().await?;
        }

        for descriptor in discovered {
            let Some(device) = DeviceCtl::descriptor_to_identity(&descriptor) else {
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

            let tx = self
                .ctl_tx
                .take()
                .ok_or_else(|| RouterError::Message("ingress task already started".to_string()))?;

            let edge_device_bin = state.edge_device_bin.clone();
            let socket_dir = state.socket_dir.clone();

            state
                .workers
                .spawn_edge_worker(
                    &edge_device_bin,
                    &socket_dir,
                    device,
                    tx.clone(),
                )
                .await?;
        }

        Ok(())
    }
}
