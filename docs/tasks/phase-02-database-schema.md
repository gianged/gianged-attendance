# Phase 02: Database Schema

## Objective

Set up PostgreSQL database with schema, tables, indexes, and views.

---

## Naming Conventions

| Prefix                 | Usage             | Example                          |
| ---------------------- | ----------------- | -------------------------------- |
| `pk_{table}`           | Primary key       | `pk_employees`                   |
| `fk_{table}_{ref}`     | Foreign key       | `fk_employees_department`        |
| `uq_{table}_{col}`     | Unique constraint | `uq_employees_code`              |
| `ck_{table}_{col}`     | Check constraint  | `ck_employees_gender`            |
| `idx_{table}_{col}`    | Index             | `idx_employees_department`       |
| `trg_{table}_{action}` | Trigger           | `trg_employees_update_timestamp` |
| `fn_{name}`            | Function          | `system.fn_update_timestamp`     |
| `v_{name}`             | View              | `v_daily_attendance`             |

---

## Schemas

| Schema   | Purpose                              |
| -------- | ------------------------------------ |
| `app`    | Application tables and views         |
| `system` | Functions, procedures, and utilities |

---

## Tasks

### 2.1 Create Database

```sql
CREATE DATABASE gianged_attendance;
```

### 2.2 Run Schema

Execute `database.sql` from project root:

```bash
psql -U postgres -d gianged_attendance -f database.sql
```

### 2.3 Tables Overview

**app.departments**

```
Column        | Type         | Constraints
--------------|--------------|---------------------------
id            | SERIAL       | pk_departments
name          | VARCHAR(100) | NOT NULL
parent_id     | INTEGER      | fk_departments_parent
display_order | INTEGER      | NOT NULL DEFAULT 0
is_active     | BOOLEAN      | NOT NULL DEFAULT true
created_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()
updated_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()
```

**app.employees**

```
Column        | Type         | Constraints
--------------|--------------|---------------------------
id            | SERIAL       | pk_employees
employee_code | VARCHAR(20)  | uq_employees_code, NOT NULL
full_name     | VARCHAR(100) | NOT NULL
department_id | INTEGER      | fk_employees_department
device_uid    | INTEGER      | uq_employees_device_uid
gender        | VARCHAR(10)  | ck_employees_gender
birth_date    | DATE         |
start_date    | DATE         | NOT NULL
end_date      | DATE         | ck_employees_dates
is_active     | BOOLEAN      | NOT NULL DEFAULT true
created_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()
updated_at    | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()
```

**app.attendance_logs**

```
Column      | Type         | Constraints
------------|--------------|---------------------------
id          | BIGSERIAL    | pk_attendance_logs
device_uid  | INTEGER      | NOT NULL
check_time  | TIMESTAMPTZ  | NOT NULL
verify_type | INTEGER      | ck_attendance_logs_verify_type
status      | INTEGER      | NOT NULL DEFAULT 0
source      | VARCHAR(20)  | ck_attendance_logs_source
created_at  | TIMESTAMPTZ  | NOT NULL DEFAULT NOW()
            |              | uq_attendance_logs_device_time
```

### 2.4 Indexes

| Index                                  | Table           | Columns                     | Condition                       |
| -------------------------------------- | --------------- | --------------------------- | ------------------------------- |
| `idx_departments_parent`               | departments     | parent_id                   | WHERE parent_id IS NOT NULL     |
| `idx_departments_active`               | departments     | is_active                   | WHERE is_active = true          |
| `idx_employees_department`             | employees       | department_id               | WHERE department_id IS NOT NULL |
| `idx_employees_device_uid`             | employees       | device_uid                  | WHERE device_uid IS NOT NULL    |
| `idx_employees_active`                 | employees       | is_active                   | WHERE is_active = true          |
| `idx_employees_code`                   | employees       | employee_code               |                                 |
| `idx_attendance_logs_device_uid`       | attendance_logs | device_uid                  |                                 |
| `idx_attendance_logs_check_time`       | attendance_logs | check_time                  |                                 |
| `idx_attendance_logs_device_time_desc` | attendance_logs | device_uid, check_time DESC |                                 |
| `idx_attendance_logs_date`             | attendance_logs | DATE(check_time)            |                                 |

### 2.5 Views

| View                              | Description                                            |
| --------------------------------- | ------------------------------------------------------ |
| `app.v_attendance_details`        | Attendance logs with employee and department names     |
| `app.v_daily_attendance`          | Daily summary per employee with work_hours calculation |
| `app.v_employees_with_department` | Employees with department name joined                  |

### 2.6 Triggers

| Trigger                            | Table       | Action                                        |
| ---------------------------------- | ----------- | --------------------------------------------- |
| `trg_departments_update_timestamp` | departments | BEFORE UPDATE -> system.fn_update_timestamp() |
| `trg_employees_update_timestamp`   | employees   | BEFORE UPDATE -> system.fn_update_timestamp() |

### 2.7 Verify Schema

```sql
-- Check tables
SELECT table_name FROM information_schema.tables
WHERE table_schema = 'app';

-- Check constraints
SELECT conname, contype FROM pg_constraint
WHERE connamespace = 'app'::regnamespace;

-- Check indexes
SELECT indexname FROM pg_indexes
WHERE schemaname = 'app';

-- Check views
SELECT viewname FROM pg_views
WHERE schemaname = 'app';

-- Check functions
SELECT routine_name FROM information_schema.routines
WHERE routine_schema = 'system';
```

---

## Deliverables

- [x] Database created
- [x] All tables with proper constraints (pk*, fk*, uq*, ck*)
- [x] All indexes created (idx\_)
- [x] All views created (v\_)
- [x] Triggers working (trg\_)
- [x] Functions deployed (system.)
