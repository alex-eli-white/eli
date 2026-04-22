

//CONSTS
const HZ_PER_MHZ: f64 = 1_000_000.0;
const HOTSPOT_REPRIORITIZE_RADIUS_HZ: f64 = 1_500_000.0;
const HOTSPOT_REPRIORITIZE_WEIGHT: f32 = 0.75;

const SCANNER_SLEEP_TIME_MS: u64 = 100;

const POWER_EPSILON: f32 = 1.0e-12;


//TYPES
pub type EdgeResult<T> = Result<T, crate::edge_error::EdgeError>;

pub mod helpers;

pub mod scanner;

pub mod ipc;

pub mod edge_error;
