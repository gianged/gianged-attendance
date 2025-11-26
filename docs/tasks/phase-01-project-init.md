# Phase 01: Project Initialization

## Objective

Initialize the Rust project with Cargo.toml and basic directory structure.

---

## Tasks

### 1.1 Create Project

```bash
cargo init gianged-attendance
```

### 1.2 Cargo.toml

```toml
[package]
name = "gianged-attendance"
version = "0.1.0"
edition = "2021"
description = "Mini ERP desktop app for staff and attendance management"
authors = ["GiangEd"]

[dependencies]
# Async runtime (rustls, no OpenSSL)
tokio = { version = "1", features = ["full", "rt-multi-thread"] }

# Database
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-rustls",
    "postgres",
    "chrono",
    "uuid"
]}

# HTTP client
reqwest = { version = "0.12", default-features = false, features = [
    "rustls-tls",
    "cookies",
    "json"
]}

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# GUI
eframe = "0.29"
egui_extras = { version = "0.29", features = ["datepicker"] }

# Excel export
rust_xlsxwriter = "0.79"

# Config
directories = "5"

[build-dependencies]
winres = "0.1"
```

### 1.3 Directory Structure

```
gianged-attendance/
├── Cargo.toml
├── build.rs
├── database.sql
├── config.example.toml
├── assets/
│   └── icon.ico
├── docs/
│   ├── overview.md
│   ├── tasks/
│   └── reference-data/
└── src/
    ├── main.rs
    └── lib.rs
```

### 1.4 Basic main.rs

```rust
fn main() {
    println!("GiangEd Attendance - Starting...");
}
```

### 1.5 Basic lib.rs

```rust
pub mod config;
pub mod error;
pub mod models;
pub mod db;
pub mod client;
pub mod sync;
pub mod export;
pub mod ui;
```

---

## Deliverables

- [x] Cargo.toml with all dependencies
- [x] Basic directory structure
- [x] Empty module files created
- [x] Project compiles with `cargo check`
