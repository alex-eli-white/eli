use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use soapysdr_sys::SoapySDRRange;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::router::flux::event_fanout::{new_router_broadcast, RouterBroadcast};
use crate::router::flux::state::RouterState;
use crate::router::genesis::rtl_genesis::RtlSdrDiscovery;
use crate::router::registries::reg_vanilla::{
    ControlLease, DeviceBackend, DeviceDescriptor, DeviceDiscovery, DeviceIdentity,
};
use crate::router::registries::worker_registry::now_ms;
use crate::router::vanilla::{EventKind, RouterEvent};
use crate::{RouterError, RouterResult};

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
        self.spawn_debug_observer().await;

        loop {
            self.reconcile_once().await?;
            tokio::time::sleep(self.discovery_interval).await;
        }
    }

    async fn ensure_socket_dir(&self) -> RouterResult<()> {
        tokio::fs::create_dir_all(&self.socket_dir).await?;
        println!("[router] edge binary: {:?}", self.edge_device_bin);
        println!("[router] socket dir: {:?}", self.socket_dir);
        println!("[router] socket dir exists? {}", self.socket_dir.exists());
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
                state_guard
                    .workers
                    .update_worker_running(&event.worker_id);
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
                        println!(
                            "[router] worker={} source={} kind={:?}",
                            event.worker_id,
                            event.source_id,
                            event.kind()
                        );
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

            println!(
                "[router] spawning worker for backend={} serial={} label={:?} product={:?}",
                device.backend,
                device.serial_number,
                descriptor.label,
                descriptor.product,
            );

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
        if let Some(lease) = &state.control_lease {
            if lease.controller_id == controller_id {
                state.control_lease = None;
            }
        }
    }
}
