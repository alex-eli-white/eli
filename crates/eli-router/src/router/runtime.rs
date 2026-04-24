use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex, RwLock};

use eli_protocol::router_vanilla::cmd_vanilla::RouterEvent;
use eli_protocol::router_vanilla::result_vanilla::RouterResult;

use crate::router::flux::event_fanout::new_router_broadcast;
use crate::router::flux::state::RouterState;
use crate::router::runspec::control::IoCtl;

pub struct RouterRuntime {
    pub control_server: IoCtl,
    pub router_state: Arc<Mutex<RouterState>>,
    pub discovery_interval: Duration,
}

impl RouterRuntime {
    pub fn new(
        socket_dir: PathBuf,
        edge_device_bin: PathBuf,
        device_discovery_interval_secs: u64,
    ) -> Self {
        let broadcaster = new_router_broadcast(1024);
        let state = Arc::new(Mutex::new(RouterState::new(broadcaster, socket_dir.clone(), edge_device_bin)));
        let (ingress_tx, ingress_rx) = mpsc::channel(1024);
        let control_server = IoCtl::new(socket_dir, state.clone(), Option::from(ingress_rx), Option::from(ingress_tx));


        Self {
            control_server,
            router_state:state,
            discovery_interval: Duration::from_secs(device_discovery_interval_secs),
        }
    }

    pub async fn run(&mut self) -> RouterResult<()> {
        self.control_server.ensure_socket_dir().await?;
        self.control_server.spawn_control_listener()?;
        self.control_server.spawn_event_ingress_task()?;
        self.control_server.spawn_debug_observer().await;

        self.control_server.spawn_from_device().await?;

        loop {
            tokio::time::sleep(self.discovery_interval).await;
            self.control_server.spawn_from_device().await?;
        }
    }
}