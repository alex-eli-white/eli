// use crate::RouterResult;
//
// #[derive(Debug, Clone)]
// pub struct DeviceDescriptor {
//     pub device_id: String,
//     pub backend: DeviceBackend,
//     pub label: String,
//     pub capabilities: DeviceCapabilities,
// }
//
// #[derive(Debug, Clone)]
// pub enum DeviceBackend {
//     RtlSdr,
//     Soapy,
//     Pluto,
// }
//
// #[derive(Debug, Clone)]
// pub struct DeviceCapabilities {
//     pub rx_channels: u8,
//     pub tx_channels: u8,
//     pub full_duplex: bool,
//     pub min_freq_hz: Option<f64>,
//     pub max_freq_hz: Option<f64>,
//     pub max_sample_rate_hz: Option<f64>,
// }
//
//
//
//
//
//
// pub trait DeviceDiscovery {
//     fn discover(&self) -> RouterResult<Vec<DeviceDescriptor>>;
// }
//
// use std::collections::HashMap;
// use crate::types::*;
//
// pub struct DeviceRegistry {
//     devices: HashMap<String, DeviceDescriptor>,
// }
//
// impl DeviceRegistry {
//     pub fn new() -> Self {
//         Self {
//             devices: HashMap::new(),
//         }
//     }
//
//     pub fn register(&mut self, device: DeviceDescriptor) {
//         self.devices.insert(device.device_id.clone(), device);
//     }
//
//     pub fn all(&self) -> Vec<DeviceDescriptor> {
//         self.devices.values().cloned().collect()
//     }
// }