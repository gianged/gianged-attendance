# Phase 06: DTOs and Business Logic

## Objective

Define DTOs (Data Transfer Objects) for create/update operations and view query results. Entity models are auto-generated from the database.

---

## Prerequisites

Generate entities from database first:

```bash
sea-orm-cli generate entity \
    -u postgres://user:pass@localhost/gianged_attendance \
    -o src/entities \
    --with-serde both \
    --date-time-crate chrono
```

This creates `src/entities/` with:
- `mod.rs`
- `departments.rs`
- `employees.rs`
- `attendance_logs.rs`

**Never modify these files manually** - they are regenerated on schema changes.

---

## Tasks

### 6.1 Create Models Directory

```
src/models/
├── mod.rs
├── department.rs
├── employee.rs
└── attendance.rs
```

### 6.2 Department DTOs

**`src/models/department.rs`**

```rust
use serde::{Deserialize, Serialize};

/// DTO for creating a department
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDepartment {
    pub name: String,
    pub parent_id: Option<i32>,
    pub display_order: i32,
}

/// DTO for updating a department
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateDepartment {
    pub name: Option<String>,
    pub parent_id: Option<Option<i32>>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
}
```

### 6.3 Employee DTOs

**`src/models/employee.rs`**

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// DTO for creating an employee
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmployee {
    pub employee_code: String,
    pub full_name: String,
    pub department_id: Option<i32>,
    pub device_uid: Option<i32>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub start_date: NaiveDate,
}

/// DTO for updating an employee
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateEmployee {
    pub employee_code: Option<String>,
    pub full_name: Option<String>,
    pub department_id: Option<Option<i32>>,
    pub device_uid: Option<Option<i32>>,
    pub gender: Option<Option<String>>,
    pub birth_date: Option<Option<NaiveDate>>,
    pub start_date: Option<NaiveDate>,
    pub is_active: Option<bool>,
}
```

### 6.4 Attendance DTOs and View Models

**`src/models/attendance.rs`**

```rust
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

/// DTO for creating an attendance log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttendanceLog {
    pub device_uid: i32,
    pub check_time: DateTime<Utc>,
    pub verify_type: i32,
    pub status: i32,
    pub source: String,
}

/// Daily attendance summary (from v_daily_attendance view)
#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct DailyAttendance {
    pub employee_id: i32,
    pub employee_code: String,
    pub full_name: String,
    pub department_id: Option<i32>,
    pub department_name: Option<String>,
    pub work_date: NaiveDate,
    pub first_check: DateTime<Utc>,
    pub last_check: DateTime<Utc>,
    pub check_count: i64,
    pub work_hours: Option<f64>,
}

/// Verify type constants
pub mod verify_type {
    pub const FINGERPRINT: i32 = 2;
    pub const CARD: i32 = 101;

    pub fn name(code: i32) -> &'static str {
        match code {
            FINGERPRINT => "fingerprint",
            CARD => "card",
            _ => "unknown",
        }
    }
}

impl DailyAttendance {
    /// Calculate work duration in hours (if not from view)
    pub fn calculate_work_hours(&self) -> f64 {
        let duration = self.last_check - self.first_check;
        duration.num_minutes() as f64 / 60.0
    }
}
```

### 6.5 Module Export

**`src/models/mod.rs`**

```rust
pub mod department;
pub mod employee;
pub mod attendance;

pub use department::{CreateDepartment, UpdateDepartment};
pub use employee::{CreateEmployee, UpdateEmployee};
pub use attendance::{CreateAttendanceLog, DailyAttendance, verify_type};
```

---

## Notes

- Entity structs come from `src/entities/` (generated)
- This module only contains DTOs and view result types
- Use `FromQueryResult` for custom query results (views)
- Entity `Model` types are used for read operations

---

## Deliverables

- [ ] Generate entities with sea-orm-cli
- [ ] Department DTOs (Create, Update)
- [ ] Employee DTOs (Create, Update)
- [ ] Attendance DTOs and DailyAttendance view model
- [ ] Module exports
