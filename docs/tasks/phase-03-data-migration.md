# Phase 03: Data Migration

## Objective

Migrate reference data from old SQL Server format to new PostgreSQL schema.

---

## Tasks

### 3.1 Migrate Departments

Source: `docs/reference-data/db_tblBoPhan.sql`

```sql
-- Create temporary staging table
CREATE TEMP TABLE tmp_departments (
    BPMa INTEGER,
    BPTen VARCHAR(100),
    BPMaCha INTEGER,
    BPUuTien INTEGER,
    BPHienThiBC BOOLEAN
);

-- Insert old data (manually convert the INSERT statements)
-- Then migrate to new schema:

INSERT INTO app.departments (id, name, parent_id, display_order, is_active)
SELECT
    BPMa,
    TRIM(BPTen),
    NULLIF(BPMaCha, 0),
    BPUuTien,
    BPHienThiBC
FROM tmp_departments;

-- Reset sequence
SELECT setval('app.departments_id_seq', (SELECT COALESCE(MAX(id), 0) FROM app.departments));
```

### 3.2 Migrate Employees

Source: `docs/reference-data/db_tblNhanVien.sql`

```sql
-- Create temporary staging table with essential fields
CREATE TEMP TABLE tmp_employees (
    NVMa INTEGER,
    NVMaNV VARCHAR(20),
    NVHoTen VARCHAR(100),
    NVMaBP INTEGER,
    NVSoID VARCHAR(10),
    NVGioiTinh BOOLEAN,
    NVNgaySinh TIMESTAMP,
    NVNgayVao TIMESTAMP,
    NVNgayRa TIMESTAMP
);

-- Insert and migrate:
INSERT INTO app.employees (
    id, employee_code, full_name, department_id, device_uid,
    gender, birth_date, start_date, end_date, is_active
)
SELECT
    NVMa,
    TRIM(NVMaNV),
    TRIM(NVHoTen),
    NULLIF(NVMaBP, 0),
    CAST(NULLIF(TRIM(NVSoID), '') AS INTEGER),
    CASE WHEN NVGioiTinh THEN 'male' ELSE 'female' END,
    CASE WHEN NVNgaySinh::DATE = '1980-01-01' THEN NULL ELSE NVNgaySinh::DATE END,
    NVNgayVao::DATE,
    CASE WHEN NVNgayRa::DATE = '9990-12-31' THEN NULL ELSE NVNgayRa::DATE END,
    true
FROM tmp_employees
WHERE NVMaNV NOT LIKE 'G%';  -- Skip guest accounts

-- Reset sequence
SELECT setval('app.employees_id_seq', (SELECT COALESCE(MAX(id), 0) FROM app.employees));
```

### 3.3 Migrate Attendance (Optional)

Source: `docs/reference-data/attlog.dat`

This would be done via application code to parse the TSV format.

### 3.4 Verify Migration

```sql
-- Check counts
SELECT 'departments' as table_name, COUNT(*) as count FROM app.departments
UNION ALL
SELECT 'employees', COUNT(*) FROM app.employees
UNION ALL
SELECT 'attendance_logs', COUNT(*) FROM app.attendance_logs;

-- Check sample data
SELECT * FROM app.departments LIMIT 5;
SELECT * FROM app.employees LIMIT 5;
```

---

## Deliverables

- [ ] Departments migrated (12 records)
- [ ] Employees migrated (~168 records, minus guests)
- [ ] Sequences reset correctly
- [ ] Data integrity verified
