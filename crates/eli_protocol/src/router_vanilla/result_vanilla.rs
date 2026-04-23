use std::fmt::{Display, Formatter};

pub type RouterResult<T> = Result<T, RouterError>;


#[derive(Debug)]
pub enum RouterError {
    Io(std::io::Error),
    JoinError(tokio::task::JoinError),
    Serde(serde_json::Error),
    Message(String),
    Soapysdr(soapysdr::Error),
}

impl From<std::io::Error> for RouterError {
    fn from(value: std::io::Error) -> Self {
        RouterError::Io(value)
    }
}

impl From<tokio::task::JoinError> for RouterError {
    fn from(value: tokio::task::JoinError) -> Self {
        RouterError::JoinError(value)
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(value: serde_json::Error) -> Self {
        RouterError::Serde(value)
    }
}

impl From<String> for RouterError {
    fn from(value: String) -> Self {
        RouterError::Message(value)
    }
}

impl From<&str> for RouterError {
    fn from(value: &str) -> Self {
        RouterError::Message(value.to_string())
    }
}

impl From<soapysdr::Error> for RouterError {
    fn from(value: soapysdr::Error) -> Self {
        RouterError::Soapysdr(value)
    }
}

impl std::error::Error for RouterError {}

impl Display for RouterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterError::Io(e) => write!(f, "IO error: {}", e),
            RouterError::JoinError(e) => write!(f, "Join error: {}", e),
            RouterError::Serde(e) => write!(f, "Serialization error: {}", e),
            RouterError::Message(msg) => write!(f, "{}", msg),
            RouterError::Soapysdr(e) => write!(f, "Soapysdr error: {}", e),
        }
    }
}
