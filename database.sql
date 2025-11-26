-- =============================================================================
-- Gianged Attendance - PostgreSQL Database Schema
-- =============================================================================
-- Version: 1.1.0
-- Description: Mini ERP for staff, department, and attendance management
--
-- Naming Conventions:
--   pk_{table}                    - Primary key
--   fk_{table}_{ref_table}        - Foreign key
--   uq_{table}_{column}           - Unique constraint
--   ck_{table}_{column}           - Check constraint
--   idx_{table}_{column(s)}       - Index
--   trg_{table}_{action}          - Trigger
--   fn_{name}                     - Function
--   v_{name}                      - View
--
-- Schemas:
--   app     - Application tables and views
--   system  - Functions, procedures, and utilities
-- =============================================================================

-- =============================================================================
-- SCHEMAS
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS app;
CREATE SCHEMA IF NOT EXISTS system;

-- =============================================================================
-- TABLES
-- =============================================================================

-- -----------------------------------------------------------------------------
-- Table: app.departments
-- Description: Company departments/divisions
-- Migrated from: tblBoPhan (12 records)
-- -----------------------------------------------------------------------------
CREATE TABLE app.departments (
    id              SERIAL,
    name            VARCHAR(100) NOT NULL,
    parent_id       INTEGER,
    display_order   INTEGER NOT NULL DEFAULT 0,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Primary Key
    CONSTRAINT pk_departments PRIMARY KEY (id),

    -- Foreign Keys
    CONSTRAINT fk_departments_parent FOREIGN KEY (parent_id)
        REFERENCES app.departments(id)
        ON DELETE SET NULL
        ON UPDATE CASCADE
);

COMMENT ON TABLE app.departments IS 'Company departments/divisions';
COMMENT ON COLUMN app.departments.id IS 'Primary key, auto-increment';
COMMENT ON COLUMN app.departments.name IS 'Department name';
COMMENT ON COLUMN app.departments.parent_id IS 'Self-referencing FK for hierarchical structure';
COMMENT ON COLUMN app.departments.display_order IS 'Sort order for UI display';
COMMENT ON COLUMN app.departments.is_active IS 'Soft delete flag';
COMMENT ON COLUMN app.departments.created_at IS 'Record creation timestamp';
COMMENT ON COLUMN app.departments.updated_at IS 'Record last update timestamp';

-- -----------------------------------------------------------------------------
-- Table: app.employees
-- Description: Employee records
-- Migrated from: tblNhanVien (168 records, essential fields only)
-- -----------------------------------------------------------------------------
CREATE TABLE app.employees (
    id              SERIAL,
    employee_code   VARCHAR(20) NOT NULL,
    full_name       VARCHAR(100) NOT NULL,
    department_id   INTEGER,
    device_uid      INTEGER,
    gender          VARCHAR(10),
    birth_date      DATE,
    start_date      DATE NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Primary Key
    CONSTRAINT pk_employees PRIMARY KEY (id),

    -- Foreign Keys
    CONSTRAINT fk_employees_department FOREIGN KEY (department_id)
        REFERENCES app.departments(id)
        ON DELETE SET NULL
        ON UPDATE CASCADE,

    -- Unique Constraints
    CONSTRAINT uq_employees_code UNIQUE (employee_code),
    CONSTRAINT uq_employees_device_uid UNIQUE (device_uid),

    -- Check Constraints
    CONSTRAINT ck_employees_gender CHECK (gender IN ('male', 'female', 'other'))
);

COMMENT ON TABLE app.employees IS 'Employee records';
COMMENT ON COLUMN app.employees.id IS 'Primary key, auto-increment';
COMMENT ON COLUMN app.employees.employee_code IS 'Unique employee code (e.g., S001, S002)';
COMMENT ON COLUMN app.employees.full_name IS 'Employee full name';
COMMENT ON COLUMN app.employees.department_id IS 'FK to departments';
COMMENT ON COLUMN app.employees.device_uid IS 'Fingerprint scanner device ID';
COMMENT ON COLUMN app.employees.gender IS 'Gender: male, female, other';
COMMENT ON COLUMN app.employees.birth_date IS 'Date of birth';
COMMENT ON COLUMN app.employees.start_date IS 'Employment start date';
COMMENT ON COLUMN app.employees.is_active IS 'Active employee flag (false = inactive/terminated)';
COMMENT ON COLUMN app.employees.created_at IS 'Record creation timestamp';
COMMENT ON COLUMN app.employees.updated_at IS 'Record last update timestamp';

-- -----------------------------------------------------------------------------
-- Table: app.attendance_logs
-- Description: Attendance check-in/out records from fingerprint scanner
-- Migrated from: attlog.dat (fingerprint scanner export)
-- -----------------------------------------------------------------------------
CREATE TABLE app.attendance_logs (
    id              BIGSERIAL,
    device_uid      INTEGER NOT NULL,
    check_time      TIMESTAMPTZ NOT NULL,
    verify_type     INTEGER NOT NULL DEFAULT 2,
    status          INTEGER NOT NULL DEFAULT 0,
    source          VARCHAR(20) NOT NULL DEFAULT 'device',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Primary Key
    CONSTRAINT pk_attendance_logs PRIMARY KEY (id),

    -- Unique Constraints
    CONSTRAINT uq_attendance_logs_device_time UNIQUE (device_uid, check_time),

    -- Check Constraints
    CONSTRAINT ck_attendance_logs_verify_type CHECK (verify_type IN (2, 101)),
    CONSTRAINT ck_attendance_logs_source CHECK (source IN ('device', 'manual', 'import'))
);

COMMENT ON TABLE app.attendance_logs IS 'Attendance check-in/out records from fingerprint scanner';
COMMENT ON COLUMN app.attendance_logs.id IS 'Primary key, auto-increment';
COMMENT ON COLUMN app.attendance_logs.device_uid IS 'Employee device ID (maps to employees.device_uid)';
COMMENT ON COLUMN app.attendance_logs.check_time IS 'Check-in/out timestamp';
COMMENT ON COLUMN app.attendance_logs.verify_type IS 'Verification method: 2=fingerprint, 101=card';
COMMENT ON COLUMN app.attendance_logs.status IS 'Device status code (typically 0)';
COMMENT ON COLUMN app.attendance_logs.source IS 'Data source: device, manual, import';
COMMENT ON COLUMN app.attendance_logs.created_at IS 'Record creation timestamp';

-- =============================================================================
-- FUNCTIONS
-- =============================================================================

-- -----------------------------------------------------------------------------
-- Function: system.fn_update_timestamp()
-- Description: Trigger function to auto-update updated_at column
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION system.fn_update_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION system.fn_update_timestamp() IS 'Trigger function to auto-update updated_at column';

-- -----------------------------------------------------------------------------
-- Function: system.fn_date_from_timestamptz(TIMESTAMPTZ)
-- Description: Extract date from timestamptz in local timezone (immutable)
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION system.fn_date_from_timestamptz(ts TIMESTAMPTZ)
RETURNS DATE AS $$
    SELECT (ts AT TIME ZONE 'Asia/Ho_Chi_Minh')::DATE;
$$ LANGUAGE sql IMMUTABLE STRICT;

COMMENT ON FUNCTION system.fn_date_from_timestamptz(TIMESTAMPTZ) IS 'Extract date from timestamptz in local timezone (immutable for indexing)';

-- =============================================================================
-- INDEXES
-- =============================================================================

-- Departments
CREATE INDEX idx_departments_parent ON app.departments(parent_id)
    WHERE parent_id IS NOT NULL;

CREATE INDEX idx_departments_active ON app.departments(is_active)
    WHERE is_active = true;

-- Employees
CREATE INDEX idx_employees_department ON app.employees(department_id)
    WHERE department_id IS NOT NULL;

CREATE INDEX idx_employees_device_uid ON app.employees(device_uid)
    WHERE device_uid IS NOT NULL;

CREATE INDEX idx_employees_active ON app.employees(is_active)
    WHERE is_active = true;

CREATE INDEX idx_employees_code ON app.employees(employee_code);

-- Attendance Logs
CREATE INDEX idx_attendance_logs_device_uid ON app.attendance_logs(device_uid);

CREATE INDEX idx_attendance_logs_check_time ON app.attendance_logs(check_time);

CREATE INDEX idx_attendance_logs_device_time_desc ON app.attendance_logs(device_uid, check_time DESC);

CREATE INDEX idx_attendance_logs_date ON app.attendance_logs(system.fn_date_from_timestamptz(check_time));

-- =============================================================================
-- TRIGGERS
-- =============================================================================

-- Departments: Auto-update updated_at
CREATE TRIGGER trg_departments_update_timestamp
    BEFORE UPDATE ON app.departments
    FOR EACH ROW
    EXECUTE FUNCTION system.fn_update_timestamp();

-- Employees: Auto-update updated_at
CREATE TRIGGER trg_employees_update_timestamp
    BEFORE UPDATE ON app.employees
    FOR EACH ROW
    EXECUTE FUNCTION system.fn_update_timestamp();

-- =============================================================================
-- VIEWS
-- =============================================================================

-- -----------------------------------------------------------------------------
-- View: app.v_attendance_details
-- Description: Attendance logs with employee and department names
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW app.v_attendance_details AS
SELECT
    al.id,
    al.device_uid,
    e.id AS employee_id,
    e.employee_code,
    e.full_name,
    e.department_id,
    d.name AS department_name,
    al.check_time,
    al.verify_type,
    CASE al.verify_type
        WHEN 2 THEN 'fingerprint'
        WHEN 101 THEN 'card'
        ELSE 'unknown'
    END AS verify_type_name,
    al.status,
    al.source,
    al.created_at
FROM app.attendance_logs al
LEFT JOIN app.employees e ON al.device_uid = e.device_uid
LEFT JOIN app.departments d ON e.department_id = d.id;

COMMENT ON VIEW app.v_attendance_details IS 'Attendance logs with employee and department names';

-- -----------------------------------------------------------------------------
-- View: app.v_daily_attendance
-- Description: Daily attendance summary per employee
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW app.v_daily_attendance AS
SELECT
    e.id AS employee_id,
    e.employee_code,
    e.full_name,
    e.department_id,
    d.name AS department_name,
    DATE(al.check_time) AS work_date,
    MIN(al.check_time) AS first_check,
    MAX(al.check_time) AS last_check,
    COUNT(*) AS check_count,
    EXTRACT(EPOCH FROM (MAX(al.check_time) - MIN(al.check_time))) / 3600.0 AS work_hours
FROM app.attendance_logs al
JOIN app.employees e ON al.device_uid = e.device_uid
LEFT JOIN app.departments d ON e.department_id = d.id
GROUP BY e.id, e.employee_code, e.full_name, e.department_id, d.name, DATE(al.check_time);

COMMENT ON VIEW app.v_daily_attendance IS 'Daily attendance summary per employee';

-- -----------------------------------------------------------------------------
-- View: app.v_employees_with_department
-- Description: Employees with department name joined
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW app.v_employees_with_department AS
SELECT
    e.id,
    e.employee_code,
    e.full_name,
    e.department_id,
    d.name AS department_name,
    e.device_uid,
    e.gender,
    e.birth_date,
    e.start_date,
    e.is_active,
    e.created_at,
    e.updated_at
FROM app.employees e
LEFT JOIN app.departments d ON e.department_id = d.id;

COMMENT ON VIEW app.v_employees_with_department IS 'Employees with department name joined';

-- =============================================================================
-- END OF SCHEMA
-- =============================================================================
