use std::sync::{Arc};
use soapysdr_sys::SoapySDRRange;
use tokio::sync::{Mutex, RwLock};
use eli_protocol::router_vanilla::device_vanilla::{
    ControlLease, DeviceDescriptor, DeviceDiscovery, DeviceIdentity,
};
use eli_protocol::router_vanilla::result_vanilla::RouterResult;
use crate::router::flux::state::RouterState;
use crate::router::genesis::rtl_genesis::RtlSdrDiscovery;
use crate::router::registries::worker_registry::now_ms;
use crate::router::runtime::RouterRuntime;



pub struct DeviceCtl;

impl DeviceCtl {

    pub fn discover_devices() -> RouterResult<Vec<DeviceDescriptor<Vec<SoapySDRRange>>>> {
        let rtl = RtlSdrDiscovery;
        rtl.discover()
    }

    pub fn descriptor_to_identity(
        descriptor: &DeviceDescriptor<Vec<SoapySDRRange>>,
    ) -> Option<DeviceIdentity> {
        let serial_number = descriptor.serial_number.clone()?;

        Some(DeviceIdentity {
            backend: descriptor.backend.clone(),
            serial_number,
        })
    }

    pub async fn try_claim_control(&self, controller_id: impl Into<String>, state: &mut Arc<Mutex<RouterState>>) -> bool {
        let controller_id = controller_id.into();
        let mut state = state.lock().await;

        if state.control_lease.is_some() {
            return false;
        }

        state.control_lease = Some(ControlLease {
            controller_id,
            issued_at_ms: now_ms(),
        });

        true
    }

    pub async fn release_control(&self, controller_id: &str, state: &Arc<Mutex<RouterState>>) {
        let mut state = state.lock().await;

        if let Some(lease) = &state.control_lease
            && lease.controller_id == controller_id
        {
            state.control_lease = None;
        }
    }
}