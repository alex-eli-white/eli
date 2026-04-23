use std::fmt::{Display, Formatter};
use std::io::Error;

#[derive(Debug)]
pub enum EdgeError {
    Io(std::io::Error),
    ErrorMessage(String),
    JoinError(tokio::task::JoinError),
    RtlSdrDeviceNotFound(String),
    Soapy(String)

}

impl EdgeError {
    pub fn msg(msg: String) -> Self {
        EdgeError::Io(Error::new(std::io::ErrorKind::Other, msg))
    }
}
impl From<std::io::Error> for EdgeError {
    fn from(value: Error) -> Self {
        EdgeError::ErrorMessage(value.to_string())
    }
}

impl From<tokio::task::JoinError> for EdgeError {
    fn from(value: tokio::task::JoinError) -> Self {
        EdgeError::JoinError(value)
    }
}

impl From<String> for EdgeError {
    fn from(value: String) -> Self {
        EdgeError::ErrorMessage(value)
    }
}

impl From<serde_json::Error> for EdgeError {
      fn from(value: serde_json::Error) -> Self {
          EdgeError::Io(std::io::Error::new(std::io::ErrorKind::Other, value.to_string()))
      }
}

impl From<&str> for EdgeError {
    fn from(value: &str) -> Self {
        EdgeError::ErrorMessage(value.to_string())
    }
}

impl From<soapysdr::Error> for EdgeError{
    fn from(value: soapysdr::Error) -> Self {
        EdgeError::Soapy(value.to_string())
    }
}

impl Display for EdgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        match self {
            EdgeError::Io(e) => write!(f, "io error: {}", e),
            EdgeError::ErrorMessage(e) => write!(f, "error message: {}", e),
            EdgeError::JoinError(e) => write!(f, "join error: {}", e),
            EdgeError::RtlSdrDeviceNotFound(e) => write!(f, "rtl sdr device not found: {}", e),
            EdgeError::Soapy(e) => write!(f, "soapy error: {}", e),
        }
    }
}



impl std::error::Error for EdgeError {}