use serde::{Deserialize, Serialize};
use crate::edge_vanilla::scanner::config_vanilla::ScannerConfig;
use crate::edge_vanilla::scanner::msg_vanilla::EdgeEvent;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RouterCommand {
    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "list_workers")]
    ListWorkers,

    #[serde(rename = "stop_worker")]
    StopWorker { worker_id: String },

    #[serde(rename = "start_worker")]
    StartWorker { worker_id: String },

    #[serde(rename = "set_config")]
    SetConfig { worker_id: String, config : Box<ScannerConfig> },

}


#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RouterReply {
    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "workers")]
    Workers { worker_ids: Vec<String> },

    #[serde(rename = "ok")]
    Ok { message: String },

    #[serde(rename = "error")]
    Error { message: String },
}


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