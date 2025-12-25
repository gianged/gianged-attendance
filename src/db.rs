//! Database connection pool and operations.

pub mod attendance;
pub mod connection;
pub mod department;
pub mod employee;

pub use connection::{TableCounts, connect, get_table_counts, get_version, test_connection};
