//! ZK protocol error types.

use thiserror::Error;

/// Errors that can occur during ZK protocol communication.
#[derive(Error, Debug)]
pub enum ZkError {
    /// IO error during socket operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to establish connection to device.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Device returned invalid or unexpected response.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Timeout waiting for device response.
    #[error("Timeout waiting for response")]
    Timeout,

    /// Operation attempted without active connection.
    #[error("Device not connected")]
    NotConnected,

    /// No data available from device.
    #[error("No data available")]
    NoData,
}

/// Result type for ZK protocol operations.
pub type Result<T> = std::result::Result<T, ZkError>;
