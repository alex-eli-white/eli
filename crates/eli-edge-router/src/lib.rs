use crate::router_error::RouterError;

pub type RouterResult<T> = Result<T, RouterError>;

pub mod vanilla;

pub mod router_error;