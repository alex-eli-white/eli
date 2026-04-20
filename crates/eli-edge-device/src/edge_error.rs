use std::fmt::{Display, Formatter};
use std::io::Error;

#[derive(Debug)]
pub enum EdgeError {
    Io(std::io::Error),
    JoinError(tokio::task::JoinError),

}
impl From<std::io::Error> for EdgeError {
    fn from(value: Error) -> Self {
        todo!()
    }
}

impl From<tokio::task::JoinError> for EdgeError {
    fn from(value: tokio::task::JoinError) -> Self {
        todo!()
    }
}

impl From<String> for EdgeError {
    fn from(value: String) -> Self {
        todo!()
    }
}

impl From<serde_json::Error> for EdgeError {
      fn from(value: serde_json::Error) -> Self {
          EdgeError::Io(std::io::Error::new(std::io::ErrorKind::Other, value.to_string()))
      }
}

impl From<&str> for EdgeError {
    fn from(value: &str) -> Self {
        todo!()
    }
}

impl From<soapysdr::Error> for EdgeError{
    fn from(value: soapysdr::Error) -> Self {
        todo!()
    }
}

impl Display for EdgeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}



impl std::error::Error for EdgeError {}