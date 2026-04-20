pub type EdgeResult<T> = Result<T, edge_error::EdgeError>;
pub mod fft_analysis;

pub mod edge_error;
pub mod dwell_capture;
pub mod sweep_planner;

pub mod hit_detection;

pub mod runner;

