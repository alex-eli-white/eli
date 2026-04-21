use crate::RouterResult;

#[derive(Debug, Clone)]
pub enum DeviceBackend {
    RtlSdr,
    Soapy,
    Pluto,
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
    pub device_id: Option<String>,
    pub product: Option<String>,
    pub label: Option<String>,
    pub manufacturer: Option<String>,
    pub backend: DeviceBackend,
    pub capabilities: DeviceCapabilities<T>,
}

pub trait DeviceDiscovery {
    type RangesType;
    fn discover(&self) -> RouterResult<Vec<DeviceDescriptor<Self::RangesType>>>;
}