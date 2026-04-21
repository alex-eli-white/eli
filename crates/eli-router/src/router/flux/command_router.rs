use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;

#[derive(Debug, Clone)]
pub struct RouterEvent {
    pub worker_id: String,
    pub source_id: String,
    pub timestamp_ms: u64,
    pub event: EdgeEvent,
}