# Phase 13: Excel Export

## Objective

Implement Excel export functionality for attendance reports.

---

## Tasks

### 13.1 Create Export Module

**`src/export.rs`**

```rust
use crate::models::attendance::DailyAttendance;
use rust_xlsxwriter::{Color, Format, Workbook, XlsxError};
use std::path::Path;

/// Export daily attendance data to Excel file
pub fn export_attendance_to_excel(
    data: &[DailyAttendance],
    path: &Path,
) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Set worksheet name
    worksheet.set_name("Attendance Report")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(rust_xlsxwriter::FormatBorder::Thin);

    // Alternate row format
    let alt_row_format = Format::new()
        .set_background_color(Color::RGB(0xD9E2F3));

    // Number format for hours
    let hours_format = Format::new()
        .set_num_format("0.00");

    // Headers
    let headers = [
        "Employee Code",
        "Full Name",
        "Department",
        "Date",
        "First Check",
        "Last Check",
        "Check Count",
        "Hours",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    // Set column widths
    worksheet.set_column_width(0, 15)?;  // Employee Code
    worksheet.set_column_width(1, 30)?;  // Full Name
    worksheet.set_column_width(2, 25)?;  // Department
    worksheet.set_column_width(3, 12)?;  // Date
    worksheet.set_column_width(4, 10)?;  // First Check
    worksheet.set_column_width(5, 10)?;  // Last Check
    worksheet.set_column_width(6, 12)?;  // Check Count
    worksheet.set_column_width(7, 10)?;  // Hours

    // Data rows
    for (idx, record) in data.iter().enumerate() {
        let row = (idx + 1) as u32;

        worksheet.write_string(row, 0, &record.employee_code)?;
        worksheet.write_string(row, 1, &record.full_name)?;
        worksheet.write_string(row, 2, record.department_name.as_deref().unwrap_or(""))?;
        worksheet.write_string(row, 3, &record.work_date.to_string())?;
        worksheet.write_string(row, 4, &record.first_check.format("%H:%M:%S").to_string())?;
        worksheet.write_string(row, 5, &record.last_check.format("%H:%M:%S").to_string())?;
        worksheet.write_number(row, 6, record.check_count as f64)?;

        // Calculate hours
        let hours = record.work_hours();
        worksheet.write_number_with_format(row, 7, hours, &hours_format)?;
    }

    // Add autofilter
    let last_row = data.len() as u32;
    worksheet.autofilter(0, 0, last_row, 7)?;

    // Freeze top row
    worksheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}

/// Export employees list to Excel
pub fn export_employees_to_excel(
    employees: &[crate::models::employee::Employee],
    departments: &[crate::models::department::Department],
    path: &Path,
) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.set_name("Employees")?;

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White);

    let headers = [
        "Code",
        "Full Name",
        "Department",
        "Device UID",
        "Gender",
        "Start Date",
        "Status",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    worksheet.set_column_width(0, 12)?;
    worksheet.set_column_width(1, 30)?;
    worksheet.set_column_width(2, 25)?;
    worksheet.set_column_width(3, 12)?;
    worksheet.set_column_width(4, 10)?;
    worksheet.set_column_width(5, 12)?;
    worksheet.set_column_width(6, 10)?;

    for (idx, emp) in employees.iter().enumerate() {
        let row = (idx + 1) as u32;

        worksheet.write_string(row, 0, &emp.employee_code)?;
        worksheet.write_string(row, 1, &emp.full_name)?;

        let dept_name = emp
            .department_id
            .and_then(|id| departments.iter().find(|d| d.id == id))
            .map(|d| d.name.as_str())
            .unwrap_or("");
        worksheet.write_string(row, 2, dept_name)?;

        worksheet.write_string(
            row,
            3,
            &emp.device_uid.map(|u| u.to_string()).unwrap_or_default(),
        )?;
        worksheet.write_string(row, 4, emp.gender.as_deref().unwrap_or(""))?;
        worksheet.write_string(row, 5, &emp.start_date.to_string())?;
        worksheet.write_string(row, 6, if emp.is_active { "Active" } else { "Inactive" })?;
    }

    worksheet.autofilter(0, 0, employees.len() as u32, 6)?;
    worksheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}

/// Generate default filename for export
pub fn generate_export_filename(prefix: &str) -> String {
    let now = chrono::Local::now();
    format!("{}_{}.xlsx", prefix, now.format("%Y%m%d_%H%M%S"))
}
```

### 13.2 File Dialog Integration

For selecting save path (will be used in GUI):

```rust
/// Open save file dialog and return selected path
pub fn show_save_dialog(default_name: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("Excel Files", &["xlsx"])
        .save_file()
}
```

Note: Add `rfd = "0.14"` to Cargo.toml for native file dialogs.

---

## Deliverables

- [ ] export_attendance_to_excel function
- [ ] export_employees_to_excel function
- [ ] Header formatting
- [ ] Column widths
- [ ] Autofilter
- [ ] Freeze panes
- [ ] Work hours calculation
- [ ] generate_export_filename helper
