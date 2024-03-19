use crate::{config::ConfigError, message_header::message_header_error::MessageHeaderError};

use std::{
    error::Error,
    fmt,
    sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Debug)]
pub enum ProtocolError {
    IOError(std::io::Error),
    MessageHeaderError(MessageHeaderError),
    ConnectionError(String),
    BuildingError(String),
    ConfigError(ConfigError),
    Error(String),
}

impl Error for ProtocolError {}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProtocolError::IOError(e) => write!(f, "IO Error: {}", e),
            ProtocolError::ConnectionError(e) => write!(f, "Connection failed: {}", e),
            ProtocolError::BuildingError(e) => write!(f, "Connection failed: {}", e),
            ProtocolError::MessageHeaderError(e) => write!(f, "Message header: {}", e),
            ProtocolError::Error(e) => write!(f, "{}", e),
            ProtocolError::ConfigError(e) => write!(f, "Config file error: {}", e),
        }
    }
}

impl From<std::io::Error> for ProtocolError {
    fn from(error: std::io::Error) -> Self {
        ProtocolError::IOError(error)
    }
}

impl<'t, D> From<PoisonError<MutexGuard<'t, D>>> for ProtocolError {
    fn from(error: PoisonError<MutexGuard<'t, D>>) -> Self {
        ProtocolError::Error(format!("Failed while getting the lock: {}", error))
    }
}

impl<'t, D> From<PoisonError<RwLockWriteGuard<'t, D>>> for ProtocolError {
    fn from(error: PoisonError<RwLockWriteGuard<'t, D>>) -> Self {
        ProtocolError::Error(format!("Failed while getting the lock: {}", error))
    }
}

impl<'t, D> From<PoisonError<RwLockReadGuard<'t, D>>> for ProtocolError {
    fn from(error: PoisonError<RwLockReadGuard<'t, D>>) -> Self {
        ProtocolError::Error(format!("Failed while getting the lock: {}", error))
    }
}

impl From<MessageHeaderError> for ProtocolError {
    fn from(error: MessageHeaderError) -> Self {
        ProtocolError::MessageHeaderError(error)
    }
}

impl From<String> for ProtocolError {
    fn from(error: String) -> Self {
        ProtocolError::Error(error)
    }
}

impl From<ConfigError> for ProtocolError {
    fn from(error: ConfigError) -> Self {
        ProtocolError::ConfigError(error)
    }
}
