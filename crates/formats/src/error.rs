use thiserror::Error;

/// Error produced when a file does not match the format being read.
#[derive(Debug, Error)]
pub enum FormatError {
    #[error("unexpected end of data while reading {what} at offset {offset}")]
    UnexpectedEof { what: &'static str, offset: u64 },

    #[error("invalid {what} at offset {offset}: {detail}")]
    Invalid { what: &'static str, offset: u64, detail: String },

    #[error("limit exceeded while reading {what}: {detail}")]
    LimitExceeded { what: &'static str, detail: String },

    #[error("unsupported {what}: {detail}")]
    Unsupported { what: &'static str, detail: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl FormatError {
    pub(crate) fn invalid(what: &'static str, offset: u64, detail: impl Into<String>) -> Self {
        FormatError::Invalid { what, offset, detail: detail.into() }
    }
}
