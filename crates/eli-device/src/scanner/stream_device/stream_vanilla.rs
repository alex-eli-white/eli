use std::ops::{Deref, DerefMut};
use num_complex::Complex32;

use eli_protocol::edge_vanilla::result_vanilla::{EdgeError, EdgeResult};


pub trait DeviceStream: Send {
    fn set_frequency(&mut self, freq_hz: f64) -> EdgeResult<()>;
    fn set_sample_rate(&mut self, sample_rate_hz: f64) -> EdgeResult<()>;
    fn discard_buffers(&mut self, count: i64, timeout_us: i64) -> Result<(), EdgeError>;
    fn read_samples(&mut self, timeout_us: i64) -> Result<Vec<Complex32>, EdgeError>;
    fn activate(&mut self) -> Result<(), EdgeError>;
    fn deactivate(&mut self) -> Result<(), EdgeError>;
    fn current_sample_rate(&self) -> EdgeResult<f64>;
    fn current_frequency(&self) -> EdgeResult<f64>;
}

