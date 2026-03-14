use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid package tag: 0x{0:08X} (expected 0x9E2A83C1)")]
    InvalidTag(u32),

    #[error("Unsupported compression: 0x{0:X}")]
    UnsupportedCompression(u32),

    #[error("LZO decompression failed: {0}")]
    LzoError(String),

    #[error("Invalid name index: {0}")]
    InvalidNameIndex(i32),

    #[error("Data too short: need {need} bytes, have {have}")]
    DataTooShort { need: usize, have: usize },

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, UpkError>;
