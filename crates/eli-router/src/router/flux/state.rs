use crate::router::registries::device_registry::DeviceRegistry;
use crate::types::*;
use crate::router::registries::listener_registry::ListenerRegistry;
use crate::router::registries::worker_registry::{WorkerHandle, WorkerRegistry};




pub struct RouterState {
    pub devices: DeviceRegistry,
    pub workers: WorkerRegistry,
    pub listeners: ListenerRegistry,
}

impl Default for RouterState {
    fn default() -> Self {
        Self {
            devices: DeviceRegistry::new(),
            workers: WorkerRegistry::new(),
            listeners: ListenerRegistry::default(),
        }
    }
}

// impl RouterState {
//     pub fn new() -> Self {
//         Self {
//             devices: DeviceRegistry::new(),
//             workers: WorkerRegistry::new(),
//             listeners: ListenerRegistry::default(),
//         }
//     }
// }