use eli_protocol::router_vanilla::device_vanilla::ControlLease;
use crate::router::flux::event_fanout::RouterBroadcast;
use crate::router::registries::worker_registry::WorkerRegistry;

pub struct RouterState {
    pub workers: WorkerRegistry,
    pub broadcaster: RouterBroadcast,
    pub control_lease: Option<ControlLease>,
}

impl RouterState {
    pub fn new(broadcaster: RouterBroadcast) -> Self {
        Self {
            workers: WorkerRegistry::new(),
            broadcaster,
            control_lease: None,
        }
    }
}
