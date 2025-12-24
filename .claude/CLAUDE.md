# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Desktop mini ERP for staff and attendance management. Syncs attendance data from a ZKTeco fingerprint scanner to PostgreSQL, with Excel report export.

**Core Features:**

1. Department management (CRUD with hierarchy)
2. Staff management (CRUD with device assignment)
3. Attendance sync from ZKTeco device
4. Reports with Excel export

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
├── main.rs             # Entry point, tokio runtime setup
├── lib.rs              # Library exports
├── config.rs           # TOML config parsing
├── error.rs            # thiserror-based error types
├── entities/           # Generated SeaORM entities (DO NOT EDIT)
├── models/             # DTOs and business logic
├── db/                 # Repository layer
├── client.rs           # ZKTeco HTTP client
├── sync.rs             # Sync orchestration
├── export.rs           # Excel export
└── ui/                 # egui/eframe GUI panels
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

| Prefix                 | Usage             | Example                          |
| ---------------------- | ----------------- | -------------------------------- |
| `pk_{table}`           | Primary key       | `pk_employees`                   |
| `fk_{table}_{ref}`     | Foreign key       | `fk_employees_department`        |
| `uq_{table}_{col}`     | Unique constraint | `uq_employees_code`              |
| `ck_{table}_{col}`     | Check constraint  | `ck_employees_gender`            |
| `idx_{table}_{col}`    | Index             | `idx_employees_department`       |
| `trg_{table}_{action}` | Trigger           | `trg_employees_update_timestamp` |

PostgreSQL schemas: `app` (tables/views), `system` (functions/triggers).

### Database Tables

**app.departments**

- Hierarchical structure via `parent_id`
- `display_order` for sorting
- Soft delete via `is_active`

**app.employees**

- `employee_code`: Unique business identifier (same as scanner_uid)
- `scanner_uid`: Employee's user ID on fingerprint scanner
- `department_id`: FK to departments

**app.attendance_logs**

- `scanner_uid` + `check_time`: Unique constraint for deduplication
- `verify_type`: 2=fingerprint, 101=card
- `source`: 'device' or 'manual'

### Views for Reporting

Query using `FromQueryResult`:

- `app.v_attendance_details` - Attendance with employee/department names
- `app.v_daily_attendance` - Daily summary with work hours calculation
- `app.v_employees_with_department` - Employees joined with department

## Key Dependencies

- **sea-orm**: ORM with type-safe query builder (database-first)
- **tokio + rustls**: Async runtime (no OpenSSL)
- **reqwest**: HTTP client for ZKTeco device
- **eframe/egui**: Native GUI
- **rust_xlsxwriter**: Excel export

## ZKTeco Device Integration

- **Type**: Legacy fingerprint terminal (~10 years old)
- **IP**: 192.168.90.11, Port: 80 (HTTP)
- **Firmware**: ZK Web Server with CSL interface
- **Auth**: Session-based cookies
- **Algorithm**: Finger VX10.0

### Data Format (TSV)

```
scanner_uid    [empty]    timestamp             verify_type    status
20                        2025-11-25 07:36:58   2              0
```

| Field       | Description                        |
| ----------- | ---------------------------------- |
| scanner_uid | Employee's user ID on scanner      |
| timestamp   | Check-in/out datetime (local time) |
| verify_type | 2=fingerprint, 101=card            |
| status      | Always 0                           |

## Configuration (config.toml)

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

## Error Handling

| Error                | Handling                      |
| -------------------- | ----------------------------- |
| Device login failure | Show error, check credentials |
| Device timeout       | Retry with backoff            |
| Database connection  | Show error, check settings    |
| Duplicate records    | Skip silently (expected)      |
| Parse errors         | Log and continue              |

## Implementation Phases

Detailed task breakdowns in `docs/tasks/phase-*.md` (phases 01-22 from project init through final packaging).
