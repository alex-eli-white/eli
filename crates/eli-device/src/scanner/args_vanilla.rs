use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum DeviceKindArg {
    Rtl,
    // Pluto,
    BladeRf,
}

#[derive(Debug, Parser)]
pub struct EdgeDeviceArgs {
    #[arg(long)]
    pub worker_id: String,

    #[arg(long)]
    pub socket_path: String,

    #[arg(long)]
    pub device_index: usize,

    #[arg(long, value_enum)]
    pub device_kind: DeviceKindArg,

    #[arg(long)]
    pub serial_number: String,


}