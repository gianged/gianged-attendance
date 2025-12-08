//! Error types and handling.

use thiserror::Error;

/// Application-wide error type
#[derive(Error, Debug)]
pub enum AppError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Device authentication failed
    #[error("Device login failed: invalid credentials or device unreachable")]
    DeviceLoginFailed,

    /// Device communication timeout
    #[error("Device timeout: {0}")]
    DeviceTimeout(String),

    /// Data parsing error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Configuration error
    #[error("Config error: {0}")]
    Config(String),

    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Excel export error
    #[error("Export error: {0}")]
    Export(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Record not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// TCP connection failed
    #[error("TCP connection failed: {0}")]
    TcpConnectionFailed(String),

    /// TCP protocol error (bad checksum, invalid response)
    #[error("TCP protocol error: {0}")]
    TcpProtocolError(String),

    /// Device is busy (locked by another client)
    #[error("Device is busy")]
    TcpDeviceBusy,

    /// TCP data parsing error
    #[error("TCP invalid data: {0}")]
    TcpInvalidData(String),
}

/// Result type alias for AppError
pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    /// Create a parse error with message
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create a config error with message
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a validation error with message
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a not found error with message
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
}
