# Gianged Attendance

Desktop mini ERP application for staff and attendance management. Syncs attendance data from ZKTeco fingerprint scanners to PostgreSQL with Excel report export.

## Features

- **Department Management** - CRUD operations with hierarchical structure
- **Staff Management** - Employee records with fingerprint device assignment
- **Attendance Sync** - Download attendance logs from ZKTeco devices
- **Reports** - Daily attendance summary with Excel export

## Requirements

- Rust 1.75+ (edition 2024)
- PostgreSQL 14+
- ZKTeco fingerprint terminal (HTTP interface)

## Quick Start

### 1. Database Setup

```bash
# Create database
createdb gianged_attendance

# Apply schema
psql -d gianged_attendance -f database.sql
```

### 2. Configuration

Create `config.toml` in the application directory:

```toml
[device]
url = "http://192.168.90.11"
username = "administrator"
password = "123456"

[database]
host = "localhost"
port = 5432
name = "gianged_attendance"
username = "postgres"
password = "password"

[sync]
days = 30
max_user_id = 300
auto_enabled = false
interval_minutes = 60

[ui]
start_minimized = false
minimize_to_tray = true
```

### 3. Build and Run

```bash
cargo build --release
./target/release/gianged-attendance
```

## Technology Stack

| Component     | Technology             |
| ------------- | ---------------------- |
| Language      | Rust                   |
| GUI           | egui/eframe            |
| Async Runtime | tokio + rustls         |
| Database      | PostgreSQL via sea-orm |
| HTTP Client   | reqwest                |
| Excel Export  | rust_xlsxwriter        |

## Project Structure

```
gianged-attendance/
├── Cargo.toml
├── database.sql            # PostgreSQL schema (source of truth)
├── config.toml             # Application configuration
├── docs/
│   ├── overview.md         # Solution overview
│   └── tasks/              # Implementation phases
└── src/
    ├── main.rs             # Entry point
    ├── config.rs           # Configuration parsing
    ├── error.rs            # Error types
    ├── entities/           # Generated SeaORM entities
    ├── models/             # DTOs and business logic
    ├── db/                 # Repository layer
    ├── client.rs           # ZKTeco HTTP client
    ├── sync.rs             # Sync orchestration
    ├── export.rs           # Excel export
    └── ui/                 # GUI panels
```

## ZKTeco Device Integration

The application connects to ZKTeco fingerprint terminals via HTTP:

- **Protocol**: HTTP with session-based cookies
- **Data Format**: Tab-separated values (TSV)
- **Verification Types**: Fingerprint (2), Card (101)

## Database Schema

Uses a database-first approach with PostgreSQL:

- **app.departments** - Department hierarchy
- **app.employees** - Staff records with device mapping
- **app.attendance_logs** - Attendance records with deduplication
- **Views** - Pre-built queries for reporting

Schema changes must be made in `database.sql` first, then entities regenerated using `sea-orm-cli`.

## Development

```bash
# Type checking
cargo check

# Lint
cargo clippy

# Format
cargo fmt

# Generate entities from database
sea-orm-cli generate entity \
    -u postgres://user:pass@localhost/gianged_attendance \
    -o src/entities \
    --with-serde both \
    --date-time-crate chrono
```

## License

Private/Internal Use
