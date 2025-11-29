//! Excel export functionality.

use crate::entities::{departments, employees};
use crate::models::attendance::{AttendanceDetail, DailyAttendance};
use chrono::Local;
use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook, XlsxError};
use std::path::{Path, PathBuf};

/// Export daily attendance summary to Excel file.
/// Shows first check, last check, and work hours per employee per day.
pub fn export_attendance_summary_to_excel(data: &[DailyAttendance], path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.set_name("Attendance Report")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin);

    // Number format for hours
    let hours_format = Format::new().set_num_format("0.00");

    // Headers
    let headers = [
        "Employee Code",
        "Full Name",
        "Department",
        "Date",
        "First Check",
        "Last Check",
        "Work Hours",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    // Column widths
    worksheet.set_column_width(0, 15)?; // Employee Code
    worksheet.set_column_width(1, 30)?; // Full Name
    worksheet.set_column_width(2, 25)?; // Department
    worksheet.set_column_width(3, 12)?; // Date
    worksheet.set_column_width(4, 10)?; // First Check
    worksheet.set_column_width(5, 10)?; // Last Check
    worksheet.set_column_width(6, 12)?; // Work Hours

    // Data rows
    for (idx, record) in data.iter().enumerate() {
        let row = (idx + 1) as u32;

        worksheet.write_string(row, 0, &record.employee_code)?;
        worksheet.write_string(row, 1, &record.full_name)?;
        worksheet.write_string(row, 2, record.department_name.as_deref().unwrap_or(""))?;
        worksheet.write_string(row, 3, record.work_date.to_string())?;

        // Convert UTC to local time for display
        let first_local = record.first_check.with_timezone(&Local);
        let last_local = record.last_check.with_timezone(&Local);

        worksheet.write_string(row, 4, first_local.format("%H:%M:%S").to_string())?;
        worksheet.write_string(row, 5, last_local.format("%H:%M:%S").to_string())?;

        // Use pre-calculated work_hours if available, otherwise calculate
        let hours = record.work_hours.unwrap_or_else(|| record.calculate_work_hours());
        worksheet.write_number_with_format(row, 6, hours, &hours_format)?;
    }

    // Autofilter
    if !data.is_empty() {
        let last_row = data.len() as u32;
        worksheet.autofilter(0, 0, last_row, 6)?;
    }

    // Freeze top row
    worksheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}

/// Export detailed attendance records to Excel file.
/// Shows every individual check time for each employee.
pub fn export_attendance_detail_to_excel(data: &[AttendanceDetail], path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.set_name("Attendance Detail")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin);

    // Headers
    let headers = [
        "Employee Code",
        "Full Name",
        "Department",
        "Date",
        "Time",
        "Verify Type",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    // Column widths
    worksheet.set_column_width(0, 15)?; // Employee Code
    worksheet.set_column_width(1, 30)?; // Full Name
    worksheet.set_column_width(2, 25)?; // Department
    worksheet.set_column_width(3, 12)?; // Date
    worksheet.set_column_width(4, 10)?; // Time
    worksheet.set_column_width(5, 12)?; // Verify Type

    // Data rows
    for (idx, record) in data.iter().enumerate() {
        let row = (idx + 1) as u32;

        worksheet.write_string(row, 0, record.employee_code.as_deref().unwrap_or(""))?;
        worksheet.write_string(row, 1, record.full_name.as_deref().unwrap_or(""))?;
        worksheet.write_string(row, 2, record.department_name.as_deref().unwrap_or(""))?;

        // Convert UTC to local time for display
        let local_time = record.check_time.with_timezone(&Local);

        worksheet.write_string(row, 3, local_time.format("%Y-%m-%d").to_string())?;
        worksheet.write_string(row, 4, local_time.format("%H:%M:%S").to_string())?;
        worksheet.write_string(row, 5, &record.verify_type_name)?;
    }

    // Autofilter
    if !data.is_empty() {
        let last_row = data.len() as u32;
        worksheet.autofilter(0, 0, last_row, 5)?;
    }

    // Freeze top row
    worksheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}

/// Open save file dialog and return selected path.
pub fn show_save_dialog(default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("Excel Files", &["xlsx"])
        .save_file()
}

/// Generate default filename for export.
pub fn generate_export_filename(prefix: &str) -> String {
    let now = Local::now();
    format!("{prefix}_{ts}.xlsx", ts = now.format("%Y%m%d_%H%M%S"))
}

/// Export employees to Excel file.
pub fn export_employees_to_excel(
    employees: &[employees::Model],
    departments: &[departments::Model],
    path: &Path,
) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.set_name("Employees")?;

    // Header format
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White)
        .set_border(FormatBorder::Thin);

    // Headers
    let headers = [
        "Employee Code",
        "Full Name",
        "Department",
        "Gender",
        "Birth Date",
        "Start Date",
        "Active",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    // Column widths
    worksheet.set_column_width(0, 15)?; // Employee Code
    worksheet.set_column_width(1, 30)?; // Full Name
    worksheet.set_column_width(2, 25)?; // Department
    worksheet.set_column_width(3, 10)?; // Gender
    worksheet.set_column_width(4, 12)?; // Birth Date
    worksheet.set_column_width(5, 12)?; // Start Date
    worksheet.set_column_width(6, 8)?; // Active

    // Data rows
    for (idx, emp) in employees.iter().enumerate() {
        let row = (idx + 1) as u32;

        worksheet.write_string(row, 0, &emp.employee_code)?;
        worksheet.write_string(row, 1, &emp.full_name)?;

        // Department name
        let dept_name = emp
            .department_id
            .and_then(|id| departments.iter().find(|d| d.id == id))
            .map(|d| d.name.as_str())
            .unwrap_or("");
        worksheet.write_string(row, 2, dept_name)?;

        worksheet.write_string(row, 3, emp.gender.as_deref().unwrap_or(""))?;

        // Birth date
        if let Some(date) = emp.birth_date {
            worksheet.write_string(row, 4, date.to_string())?;
        } else {
            worksheet.write_string(row, 4, "")?;
        }

        worksheet.write_string(row, 5, emp.start_date.to_string())?;
        worksheet.write_string(row, 6, if emp.is_active { "Yes" } else { "No" })?;
    }

    // Autofilter
    if !employees.is_empty() {
        let last_row = employees.len() as u32;
        worksheet.autofilter(0, 0, last_row, 6)?;
    }

    // Freeze top row
    worksheet.set_freeze_panes(1, 0)?;

    workbook.save(path)?;
    Ok(())
}
