use thiserror::Error;

#[derive(Error, Debug)]
pub enum ObjectError {
    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    #[error("Missing property: {0}")]
    MissingProperty(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UPK error: {0}")]
    UpkError(#[from] cimmeria_upk::UpkError),
}

pub type Result<T> = std::result::Result<T, ObjectError>;
