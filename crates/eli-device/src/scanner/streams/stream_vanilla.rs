use std::ops::DerefMut;
use std::sync::Arc;
use num_complex::Complex32;

use crate::edge_error::EdgeError;

pub trait DeviceStream : Send{
    fn set_frequency(&mut self, freq_hz: f64) -> Result<(), EdgeError>;

    fn discard_buffers(&mut self, count: i64, timeout_us: i64) -> Result<(), EdgeError>;

    fn read_samples(&mut self, timeout_us: i64) -> Result<Vec<Complex32>, EdgeError>;

    fn activate(&mut self) -> Result<(), EdgeError>;
    fn deactivate(&mut self) -> Result<(), EdgeError>;
    fn current_sample_rate(&self) -> Result<f64, EdgeError>;
    fn current_frequency(&self) -> Result<f64, EdgeError>;
}

pub struct DeviceStreamWrapper(pub Arc<Box<dyn DeviceStream>>);

impl std::ops::Deref for DeviceStreamWrapper {
    type Target = Box<dyn DeviceStream>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DeviceStreamWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(&mut self.0).expect("Failed to get mutable reference")
    }
}