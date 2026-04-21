use crate::types::{DeviceDescriptor, DeviceDiscovery};
use crate::router::genesis::rtl_genesis::RtlSdrDevice;
use soapysdr_sys::SoapySDRRange;
use std::path::PathBuf;
use std::sync::Arc;
use crate::router::flux::state::RouterState;

pub struct RouterRuntime {
    pub socket_dir: PathBuf,
    pub edge_device_bin: String,
    pub state: Arc<tokio::sync::Mutex<RouterState>>,
}

impl RouterRuntime {
    pub fn new(socket_dir: impl Into<PathBuf>, edge_device_bin: impl Into<String>) -> Self {
        Self {
            socket_dir: socket_dir.into(),
            edge_device_bin: edge_device_bin.into(),
            state: RouterState::default(),
        }
    }
    pub async fn run(&mut self) -> Result<(), RouterError> {
        self.ensure_socket_dir().await?;
        self.register_debug_listener().await?;

        let discovered = self.discover_devices()?;
        self.register_devices(discovered);

        self.spawn_workers_for_devices().await?;

        futures::future::pending::<()>().await;
        Ok(())
    }
    

    pub fn register_devices<T>(&mut self, devices: Vec<crate::types::DeviceDescriptor<T>>)
    where
        T: Clone,
    {
        for device in devices {
            println!(
                "[router] discovered device id={:?} product={:?} label={:?}",
                device.device_id, device.product, device.label
            );
            // state.devices.register(...) once DeviceRegistry is generic or normalized
        }
    }
    


    pub fn discover_devices(&self) -> Result<Vec<DeviceDescriptor<Vec<SoapySDRRange>>>, RouterError> {
        let rtl = RtlSdrDevice;
        rtl.discover()
    }
}