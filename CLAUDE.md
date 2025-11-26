# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Desktop mini ERP for staff and attendance management. Syncs attendance data from a ZKTeco fingerprint scanner (legacy device at 192.168.90.11) to PostgreSQL, with Excel report export.

## Build Commands

```bash
cargo build --release    # Production build (~15 MB executable)
cargo check              # Fast type checking
cargo clippy             # Lint
cargo fmt                # Format
```

## Architecture

```
src/
├── main.rs          # Entry point, tokio runtime setup
├── config.rs        # TOML config parsing (device, database, sync, ui)
├── error.rs         # thiserror-based error types
├── models/          # Domain models (Department, Employee, Attendance)
├── db/              # Database repositories (SeaORM)
├── entities/        # Generated SeaORM entities (DO NOT EDIT)
├── client.rs        # ZKTeco HTTP client (session cookies, TSV parsing)
├── sync.rs          # Sync orchestration service
├── export.rs        # Excel export (rust_xlsxwriter)
└── ui/              # egui/eframe GUI panels
```

## Database-First Approach (Mandatory)

**The database schema (`database.sql`) is the single source of truth.**

- All schema changes happen in `database.sql` first
- Entities are generated FROM the database using `sea-orm-cli`
- Never use SeaORM migrations or code-first features
- Never modify files in `src/entities/` manually (they get overwritten)

### Entity Generation

```bash
sea-orm-cli generate entity \
    -u postgres://user:pass@localhost/gianged_attendance \
    -o src/entities \
    --with-serde both \
    --date-time-crate chrono
```

### Schema Conventions

| Prefix | Usage | Example |
|--------|-------|---------|
| `pk_{table}` | Primary key | `pk_employees` |
| `fk_{table}_{ref}` | Foreign key | `fk_employees_department` |
| `uq_{table}_{col}` | Unique constraint | `uq_employees_code` |
| `ck_{table}_{col}` | Check constraint | `ck_employees_gender` |
| `idx_{table}_{col}` | Index | `idx_employees_department` |
| `trg_{table}_{action}` | Trigger | `trg_employees_update_timestamp` |

PostgreSQL schemas: `app` (tables/views), `system` (functions/triggers).

## Key Dependencies

- **sea-orm**: ORM with type-safe query builder (database-first)
- **tokio + rustls**: Async runtime (no OpenSSL)
- **reqwest**: HTTP client for ZKTeco device
- **eframe/egui**: Native GUI
- **rust_xlsxwriter**: Excel export

## ZKTeco Device Integration

- IP: 192.168.90.11, Port: 80
- Auth: Session-based cookies
- Data format: TSV (tab-separated)
- Fields: device_uid, timestamp, verify_type (2=fingerprint, 101=card), status

## Views for Reporting

Query database views using `FromQueryResult`:
- `app.v_attendance_details` - Attendance with employee/department names
- `app.v_daily_attendance` - Daily summary with work hours
- `app.v_employees_with_department` - Employees with department joined

## Implementation Phases

Detailed task breakdowns in `docs/tasks/phase-*.md`. Current structure follows phases 01-22 from project init through final packaging.
