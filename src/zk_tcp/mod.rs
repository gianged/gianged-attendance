//! ZKTeco TCP binary protocol client.
//!
//! Implements the binary protocol on port 4370 for complete attendance data retrieval.
//! Based on the pyzk library implementation.

mod client;
mod io;
mod parser;
mod protocol;
mod transfer;
mod types;

#[cfg(test)]
mod tests;

// Re-export public API
pub use client::ZkTcpClient;
