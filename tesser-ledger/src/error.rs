use thiserror::Error;

/// Result alias for ledger operations.
pub type LedgerResult<T> = Result<T, LedgerError>;

/// Error type surfaced by ledger operations.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("invalid ledger state: {0}")]
    InvalidState(String),
}

impl From<rusqlite::Error> for LedgerError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Storage(value.to_string())
    }
}

impl From<std::io::Error> for LedgerError {
    fn from(value: std::io::Error) -> Self {
        Self::Storage(value.to_string())
    }
}

impl From<parquet::errors::ParquetError> for LedgerError {
    fn from(value: parquet::errors::ParquetError) -> Self {
        Self::Storage(value.to_string())
    }
}

impl From<arrow::error::ArrowError> for LedgerError {
    fn from(value: arrow::error::ArrowError) -> Self {
        Self::Storage(value.to_string())
    }
}
