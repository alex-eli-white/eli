use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use crate::router_vanilla::result_vanilla::RouterResult;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceBackend {
    Rtl,
    BladeRf,
}

impl DeviceBackend {
    pub fn cli_value(&self) -> &'static str {
        match self {
            DeviceBackend::Rtl => "rtl",
            DeviceBackend::BladeRf => "bladerf",
        }
    }
}

impl Display for DeviceBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cli_value())
    }
}

#[derive(Debug, Clone)]
pub struct DeviceCapabilities<T> {
    pub rx_channels: usize,
    pub tx_channels: Option<usize>,
    pub full_duplex: bool,
    pub tuner: Option<String>,
    pub sample_rate: Option<f64>,
    pub frequency_ranges: Option<T>,
}

#[derive(Debug, Clone)]
pub struct DeviceDescriptor<T> {
    pub backend: DeviceBackend,
    pub serial_number: Option<String>,
    pub product: Option<String>,
    pub label: Option<String>,
    pub manufacturer: Option<String>,
    pub capabilities: DeviceCapabilities<T>,
}

#[derive(Debug, Clone, Eq)]
pub struct DeviceIdentity {
    pub backend: DeviceBackend,
    pub serial_number: String,
}

impl PartialEq for DeviceIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.backend == other.backend && self.serial_number == other.serial_number
    }
}

impl Hash for DeviceIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.backend.hash(state);
        self.serial_number.hash(state);
    }
}

impl DeviceIdentity {
    pub fn worker_id(&self) -> String {
        format!("{}-{}", self.backend.cli_value(), self.serial_number)
    }

    pub fn socket_name(&self) -> String {
        format!("{}.sock", self.worker_id())
    }
}

#[derive(Debug, Clone)]
pub struct ControlLease {
    pub controller_id: String,
    pub issued_at_ms: u128,
}

pub trait DeviceDiscovery {
    type RangesType;
    fn discover(&self) -> RouterResult<Vec<DeviceDescriptor<Self::RangesType>>>;
}
