use std::sync::Arc;
use crate::router::flux::state::RouterState;
use crate::router_error::RouterError;

pub type RouterResult<T> = Result<T, RouterError>;

pub type SharedRouterState = Arc<tokio::sync::Mutex<RouterState>>;

pub mod router_error;

pub mod router;

pub mod types;