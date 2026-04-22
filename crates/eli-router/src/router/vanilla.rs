use eli_protocol::edge_vanilla::scanner::msg_vanilla::EdgeEvent;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventKind {
    Status,
    Record,
    Waterfall,
    IqChunk,
    SetConfig,
    Hello,
    Start,
    Stop,
    Ping,
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct RouterEvent {
    pub worker_id: String,
    pub source_id: String,
    pub timestamp_ms: u128,
    pub event: EdgeEvent,
}

impl RouterEvent {
    pub fn kind(&self) -> EventKind {
        match &self.event {
            EdgeEvent::Status(_) => EventKind::Status,
            EdgeEvent::Record(_) => EventKind::Record,
            EdgeEvent::Waterfall(_) => EventKind::Waterfall,
            EdgeEvent::IqChunk(_) => EventKind::IqChunk,
            EdgeEvent::Hello(_) => EventKind::Hello,
            EdgeEvent::Start => EventKind::Start,
            EdgeEvent::Stop => EventKind::Stop,
            EdgeEvent::Ping => EventKind::Ping,
            EdgeEvent::Shutdown => EventKind::Shutdown,
            EdgeEvent::SetConfig(_) => EventKind::SetConfig,
        }
    }
}
