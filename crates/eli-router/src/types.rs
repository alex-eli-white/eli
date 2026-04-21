use std::sync::Arc;
use tokio::sync::mpsc;
use crate::RouterError;
use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;

use soapysdr::*;
use crate::router::flux::state::RouterState;

// pub type RouterResult<T> = Result<T, RouterError>;
// pub type SharedRouterState = Arc<tokio::sync::Mutex<RouterState>>;


// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum EventKind {
//     Status,
//     Record,
//     Waterfall,
//     IqChunk,
// }
//
// #[derive(Debug, Clone)]
// pub struct RouterEvent {
//     pub worker_id: String,
//     pub source_id: String,
//     pub timestamp_ms: u64,
//     pub event: EdgeEvent,
// }
//
// impl RouterEvent {
//     pub fn kind(&self) -> EventKind {
//         match &self.event {
//             EdgeEvent::Status(_) => EventKind::Status,
//             EdgeEvent::Record(_) => EventKind::Record,
//             EdgeEvent::Waterfall(_) => EventKind::Waterfall,
//             EdgeEvent::IqChunk(_) => EventKind::IqChunk,
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct ListenerFilter {
//     pub worker_id: Option<String>,
// }
//
// impl ListenerFilter {
//     pub fn all() -> Self {
//         Self { worker_id: None }
//     }
//
//     pub fn matches(&self, event: &RouterEvent) -> bool {
//         if let Some(ref wid) = self.worker_id {
//             return &event.worker_id == wid;
//         }
//         true
//     }
// }

// #[derive(Debug, Clone)]
// pub enum DeviceBackend {
//     RtlSdr,
//     Soapy,
//     Pluto,
// }
//
// #[derive(Debug, Clone)]
// pub struct DeviceCapabilities<T> {
//     pub rx_channels: usize,
//     pub tx_channels: Option<usize>,
//     pub full_duplex: bool,
//     pub tuner: Option<String>,
//     pub sample_rate: Option<f64>,
//     pub frequency_ranges: Option<T>,
// }
//
// #[derive(Debug, Clone)]
// pub struct DeviceDescriptor<T> {
//     pub device_id: Option<String>,
//     pub product: Option<String>,
//     pub label: Option<String>,
//     pub manufacturer: Option<String>,
//     pub backend: DeviceBackend,
//     pub capabilities: DeviceCapabilities<T>,
// }
//
// pub trait DeviceDiscovery {
//     type RangesType;
//     fn discover(&self) -> RouterResult<Vec<DeviceDescriptor<Self::RangesType>>>;
// }