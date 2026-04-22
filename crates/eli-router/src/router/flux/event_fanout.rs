use tokio::sync::broadcast;

use crate::router::vanilla::RouterEvent;

pub type RouterBroadcast = broadcast::Sender<RouterEvent>;
pub type RouterBroadcastRx = broadcast::Receiver<RouterEvent>;

pub fn new_router_broadcast(capacity: usize) -> RouterBroadcast {
    let (tx, _) = broadcast::channel(capacity);
    tx
}
