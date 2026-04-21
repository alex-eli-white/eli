use std::path::Path;

use tokio::sync::mpsc;
use eli_router::router::registries::listener_registry::ListenerFilter;

use eli_router::router::flux::state::RouterState;

use clap::Parser;
use eli_router::router::vanilla::RouterRuntime;
use eli_router::router_error::RouterError;

#[derive(Debug, Parser)]
struct RouterArgs {
    #[arg(long, default_value = "/tmp/eli-router")]
    socket_dir: String,

    #[arg(long, default_value = "eli-edge-device")]
    edge_device_bin: String,
}

#[tokio::main]
async fn main() -> Result<(), RouterError> {
    let args = RouterArgs::parse();
    let mut router = RouterRuntime::new(args.socket_dir, args.edge_device_bin);
    router.run().await
}
