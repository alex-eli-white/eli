use std::path::{Path, PathBuf};
use clap::Parser;
use eli_router::router::runtime::RouterRuntime;
use eli_router::router_error::RouterError;

#[derive(Debug, Parser)]
struct RouterArgs {
    #[arg(long, default_value = "/tmp/eli-router")]
    socket_dir: PathBuf,

    #[arg(long, default_value = "eli-device")]
    edge_device_bin: PathBuf,

    #[arg(long, default_value_t = 3)]
    discovery_interval_secs: u64,
}

#[tokio::main]
async fn main() -> Result<(), RouterError> {
    let args = RouterArgs::parse();

    let mut router = RouterRuntime::new(
        args.socket_dir,
        args.edge_device_bin,
        args.discovery_interval_secs,
    );

    router.run().await
}


