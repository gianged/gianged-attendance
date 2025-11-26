# GiangEd Attendance - Solution Overview

## Project Goal

Build a desktop mini ERP application to:

1. Manage staff records (CRUD operations)
2. Manage department structure (CRUD operations)
3. Sync attendance data from ZKTeco fingerprint scanner
4. Generate attendance reports with Excel export

---

## System Context

### Device Information

- **Type**: ZKTeco fingerprint/attendance terminal (legacy, ~10 years old)
- **IP Address**: `192.168.90.11`
- **Port**: `80` (HTTP)
- **Firmware**: ZK Web Server with CSL interface
- **Authentication**: Session-based cookies
- **Algorithm**: Finger VX10.0 (ZKTeco proprietary fingerprint algorithm)

### Data Format

Attendance records are exported as tab-separated values (TSV):

```
device_uid    [empty]    timestamp             verify_type    status
20                       2025-11-25 07:36:58   2              0
```

| Field       | Description                        |
| ----------- | ---------------------------------- |
| device_uid  | Employee ID on device              |
| timestamp   | Check-in/out datetime (local time) |
| verify_type | 2 = fingerprint, 101 = card        |
| status      | Always 0                           |

---

## Database Schema

PostgreSQL database with schemas `app` (tables/views) and `system` (functions).

### Naming Conventions

| Prefix | Usage | Example |
|--------|-------|---------|
| `pk_{table}` | Primary key | `pk_employees` |
| `fk_{table}_{ref}` | Foreign key | `fk_employees_department` |
| `uq_{table}_{col}` | Unique constraint | `uq_employees_code` |
| `ck_{table}_{col}` | Check constraint | `ck_employees_gender` |
| `idx_{table}_{col}` | Index | `idx_employees_department` |
| `trg_{table}_{action}` | Trigger | `trg_employees_update_timestamp` |

### app.departments

| Column        | Type         | Constraints                     |
| ------------- | ------------ | ------------------------------- |
| id            | SERIAL       | pk_departments                  |
| name          | VARCHAR(100) | NOT NULL                        |
| parent_id     | INTEGER      | fk_departments_parent           |
| display_order | INTEGER      | NOT NULL DEFAULT 0              |
| is_active     | BOOLEAN      | NOT NULL DEFAULT true           |
| created_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()          |
| updated_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()          |

### app.employees

| Column        | Type         | Constraints                     |
| ------------- | ------------ | ------------------------------- |
| id            | SERIAL       | pk_employees                    |
| employee_code | VARCHAR(20)  | uq_employees_code, NOT NULL     |
| full_name     | VARCHAR(100) | NOT NULL                        |
| department_id | INTEGER      | fk_employees_department         |
| device_uid    | INTEGER      | uq_employees_device_uid         |
| gender        | VARCHAR(10)  | ck_employees_gender             |
| birth_date    | DATE         |                                 |
| start_date    | DATE         | NOT NULL                        |
| is_active     | BOOLEAN      | NOT NULL DEFAULT true           |
| created_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()          |
| updated_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()          |

### app.attendance_logs

| Column      | Type        | Constraints                     |
| ----------- | ----------- | ------------------------------- |
| id          | BIGSERIAL   | pk_attendance_logs              |
| device_uid  | INTEGER     | NOT NULL                        |
| check_time  | TIMESTAMPTZ | NOT NULL                        |
| verify_type | INTEGER     | ck_attendance_logs_verify_type  |
| status      | INTEGER     | NOT NULL DEFAULT 0              |
| source      | VARCHAR(20) | ck_attendance_logs_source       |
| created_at  | TIMESTAMPTZ | NOT NULL DEFAULT NOW()          |
|             |             | uq_attendance_logs_device_time  |

### Views

| View | Description |
|------|-------------|
| `app.v_attendance_details` | Attendance with employee/department names |
| `app.v_daily_attendance` | Daily summary with work_hours calculation |
| `app.v_employees_with_department` | Employees joined with department |

---

## Technology Stack

- **Language**: Rust
- **GUI Framework**: egui/eframe (pure Rust, single executable)
- **Async Runtime**: tokio with rustls (no OpenSSL)
- **Database**: PostgreSQL via sea-orm (database-first)
- **HTTP Client**: reqwest with cookie support
- **Excel Export**: rust_xlsxwriter

### Database-First Approach

The database schema (`database.sql`) is the single source of truth:
- All schema changes happen in SQL first
- Entities are generated from the database using `sea-orm-cli`
- Never modify generated entity files manually

---

## Application Features

### 1. Department Management

- List all departments with hierarchy
- Create new department
- Edit department details
- Delete department (with confirmation)
- Set parent department for hierarchy

### 2. Staff Management

- List all employees with department filter
- Search by name or employee code
- Create new employee
- Edit employee details
- Assign fingerprint device ID
- Set employment status (active/inactive)

### 3. Attendance Sync

- Connect to ZKTeco device via HTTP
- Authenticate with session cookies
- Download attendance data for date range
- Parse TSV format
- Insert records with deduplication
- Show sync progress and summary

### 4. Reports

- Daily attendance summary
- Filter by date range, department, employee
- Show first check-in, last check-out, total hours
- Export to Excel (.xlsx)

---

## Project Structure

```
gianged-attendance/
├── Cargo.toml
├── database.sql            # PostgreSQL schema
├── README.md
├── build.rs                # Windows icon/manifest
├── assets/
│   └── icon.ico
├── docs/
│   ├── overview.md         # This file
│   ├── tasks/
│   │   ├── phase-01-project-init.md
│   │   ├── phase-02-database-schema.md
│   │   ├── phase-03-data-migration.md
│   │   ├── phase-04-error-types.md
│   │   ├── phase-05-configuration.md
│   │   ├── phase-06-models.md
│   │   ├── phase-07-db-pool.md
│   │   ├── phase-08-db-department.md
│   │   ├── phase-09-db-employee.md
│   │   ├── phase-10-db-attendance.md
│   │   ├── phase-11-zkteco-client.md
│   │   ├── phase-12-sync-service.md
│   │   ├── phase-13-excel-export.md
│   │   ├── phase-14-gui-app-state.md
│   │   ├── phase-15-gui-main-window.md
│   │   ├── phase-16-gui-dashboard.md
│   │   ├── phase-17-gui-departments.md
│   │   ├── phase-18-gui-employees.md
│   │   ├── phase-19-gui-sync.md
│   │   ├── phase-20-gui-reports.md
│   │   ├── phase-21-gui-settings.md
│   │   └── phase-22-build-package.md
│   └── reference-data/     # Old database exports
├── migrations/
│   └── 001_init.sql
└── src/
    ├── main.rs             # Entry point
    ├── lib.rs              # Library exports
    ├── config.rs           # Configuration
    ├── error.rs            # Error types
    ├── entities/           # Generated SeaORM entities (DO NOT EDIT)
    │   ├── mod.rs
    │   ├── departments.rs
    │   ├── employees.rs
    │   └── attendance_logs.rs
    ├── models/             # DTOs and business logic
    │   ├── mod.rs
    │   ├── department.rs
    │   ├── employee.rs
    │   └── attendance.rs
    ├── db/                 # Repository layer
    │   ├── mod.rs
    │   ├── department.rs
    │   ├── employee.rs
    │   └── attendance.rs
    ├── client.rs           # ZKTeco HTTP client
    ├── sync.rs             # Sync orchestration
    ├── export.rs           # Excel export
    └── ui/
        ├── mod.rs
        ├── main_panel.rs
        ├── departments.rs
        ├── employees.rs
        ├── sync_panel.rs
        └── reports.rs
```

---

## Configuration File (config.toml)

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

---

## Build

```bash
cargo build --release
```

Output: `target/release/gianged-attendance.exe` (~15 MB)

---

## Error Handling

| Error                | Handling                      |
| -------------------- | ----------------------------- |
| Device login failure | Show error, check credentials |
| Device timeout       | Retry with backoff            |
| Database connection  | Show error, check settings    |
| Duplicate records    | Skip silently (expected)      |
| Parse errors         | Log and continue              |
