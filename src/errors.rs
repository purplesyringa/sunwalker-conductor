use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub enum Error {
    InvokerFailure(String),
    ConductorFailure(String),
    ConfigurationFailure(String),
    CommunicationError(String),
    UserFailure(String),
}

pub use Error::*;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}
