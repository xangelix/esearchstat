use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid file header: file too small")]
    InvalidHeader,

    #[error("Invalid file header: incorrect magic number")]
    InvalidMagic,

    #[error("File truncated inside {0}")]
    FileTruncated(&'static str),

    #[error("Invalid string offset inside search results entry")]
    InvalidOffset,

    #[error("UTF-8 decoding error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}
