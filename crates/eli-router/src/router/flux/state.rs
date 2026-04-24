use std::path::PathBuf;
use eli_protocol::router_vanilla::device_vanilla::ControlLease;
use crate::router::flux::event_fanout::RouterBroadcast;
use crate::router::registries::worker_registry::WorkerRegistry;

pub struct RouterState {
    pub workers: WorkerRegistry,
    pub broadcaster: RouterBroadcast,
    pub control_lease: Option<ControlLease>,
    pub socket_dir: PathBuf,
    pub edge_device_bin: PathBuf,
}

impl RouterState {
    pub fn new(broadcaster: RouterBroadcast, socket_dir: PathBuf, edge_device_bin: PathBuf) -> Self {
        Self {
            workers: WorkerRegistry::new(),
            broadcaster,
            control_lease: None,
            socket_dir,
            edge_device_bin,
        }
    }
}
