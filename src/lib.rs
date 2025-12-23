pub mod client;
pub mod config;
pub mod db;
pub mod entities;
pub mod error;
pub mod export;
pub mod models;
pub mod sync;
pub mod ui;
pub mod zk;

pub use error::{AppError, Result};
