use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::router::vanilla::{EventKind, RouterEvent};
use crate::RouterResult;


#[derive(Debug, Clone, Default)]
pub struct ListenerFilter {
    pub worker_id: Option<String>,
    pub source_id: Option<String>,
    pub event_kinds: Option<Vec<EventKind>>,
}

impl ListenerFilter {
    pub fn all() -> Self {
        Self {
            worker_id: None,
            source_id: None,
            event_kinds: None,
        }
    }

    pub fn matches(&self, event: &RouterEvent) -> bool {
        if let Some(worker_id) = &self.worker_id && &event.worker_id != worker_id {
                return false;
        }

        if let Some(source_id) = &self.source_id && &event.source_id != source_id {
                return false;
        }

        let kind = event.kind();

        if let Some(event_kinds) = &self.event_kinds && !event_kinds.contains(&kind) {
            return false;
        }

        true
    }

    pub async fn register_debug_listener(&mut self) -> Result<(), RouterError> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(256);

        self.state.listeners.register(
            "debug",
            ListenerFilter::all(),
            tx,
        );

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                println!(
                    "[router] worker={} source={} kind={:?}",
                    event.worker_id,
                    event.source_id,
                    event.kind()
                );
            }
        });

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RegisteredListener {
    pub name: String,
    pub filter: ListenerFilter,
    pub tx: mpsc::Sender<RouterEvent>,
    pub dropped: Arc<AtomicU64>,
}


#[derive(Debug, Clone, Default)]
pub struct ListenerRegistry {
    listeners: Vec<RegisteredListener>,
}

impl ListenerRegistry {
    // pub fn new() -> Self {
    //     Self {
    //         listeners: Vec::new(),
    //     }
    // }

    pub fn register(
        &mut self,
        name: impl Into<String>,
        filter: ListenerFilter,
        tx: mpsc::Sender<RouterEvent>,
    ) {
        self.listeners.push(RegisteredListener {
            name: name.into(),
            filter,
            tx,
            dropped: Arc::new(AtomicU64::new(0)),
        });
    }

    pub fn publish(&self, event: RouterEvent) {
        for listener in &self.listeners {
            if listener.filter.matches(&event) {
                if listener.tx.try_send(event.clone()).is_err() {
                    listener.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}