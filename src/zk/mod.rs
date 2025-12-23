//! ZKTeco TCP protocol client (port 4370).
//!
//! Communicates with ZKTeco devices using the binary TCP protocol
//! to read attendance data directly from flash storage.
//!
//! # Example
//!
//! ```ignore
//! use gianged_attendance::zk::ZkTcpClient;
//!
//! let mut client = ZkTcpClient::connect("192.168.90.11:4370")?;
//! let records = client.get_attendance()?;
//! // client automatically disconnects on drop
//! ```

mod attendance;
mod client;
mod error;
mod protocol;

pub use attendance::AttendanceRecord;
pub use client::ZkTcpClient;
pub use error::{Result, ZkError};
