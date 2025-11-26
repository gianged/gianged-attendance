//! Database connection pool and operations.

pub mod connection;
pub mod department;

pub use connection::{TableCounts, connect, get_table_counts, get_version, test_connection};
