//! Attendance reports panel with filters and Excel export.

use chrono::{Datelike, Local, NaiveDate};
use eframe::egui::{self, RichText, ScrollArea, Ui};
use egui_phosphor::regular::{
    ARROWS_CLOCKWISE, CARET_DOUBLE_LEFT, CARET_DOUBLE_RIGHT, CARET_LEFT, CARET_RIGHT, FILE_XLS, MAGNIFYING_GLASS,
};

use super::app::{App, REPORT_PAGE_SIZE, ReportType};
use super::components::{back_button, panel_header, primary_button_with_icon, styled_button, styled_button_with_icon};

/// Parse date from multiple formats: "2000-1-1", "2000/1/1", "2000 1 1", "2000.1.1"
fn parse_flexible_date(input: &str) -> Option<NaiveDate> {
    let input = input.trim();

    // Split by common separators: - / space .
    let parts: Vec<&str> = input
        .split(['-', '/', ' ', '.'])
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() != 3 {
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    NaiveDate::from_ymd_opt(year, month, day)
}

/// Show the reports panel.
///
/// Returns `true` if the back button was clicked.
pub fn show(app: &mut App, ui: &mut Ui) -> bool {
    let mut go_back = false;

    if back_button(ui) {
        go_back = true;
    }

    panel_header(ui, "Attendance Reports");

    // Report type toggle
    ui.horizontal(|ui| {
        ui.label("Report Type:");
        ui.add_space(10.0);

        if ui
            .selectable_label(app.report_filter.report_type == ReportType::Summary, "Summary")
            .clicked()
        {
            app.report_filter.report_type = ReportType::Summary;
        }

        if ui
            .selectable_label(app.report_filter.report_type == ReportType::Detail, "Detail")
            .clicked()
        {
            app.report_filter.report_type = ReportType::Detail;
        }
    });

    ui.add_space(10.0);

    // Date range filters
    ui.horizontal(|ui| {
        ui.label("From:");
        // Check if current input is valid (flexible parsing)
        let start_valid = parse_flexible_date(&app.report_filter.start_date_input).is_some();
        let start_response = ui.add(
            egui::TextEdit::singleline(&mut app.report_filter.start_date_input)
                .desired_width(100.0)
                .hint_text("YYYY-MM-DD")
                .text_color(if start_valid {
                    ui.visuals().text_color()
                } else {
                    egui::Color32::from_rgb(220, 50, 50)
                }),
        );
        if start_response.changed()
            && let Some(date) = parse_flexible_date(&app.report_filter.start_date_input)
        {
            app.report_filter.start_date = date;
        }
        // On focus lost, normalize to YYYY-MM-DD format or reset if invalid
        if start_response.lost_focus() {
            if let Some(date) = parse_flexible_date(&app.report_filter.start_date_input) {
                app.report_filter.start_date = date;
                app.report_filter.start_date_input = date.format("%Y-%m-%d").to_string();
            } else {
                app.report_filter.start_date_input = app.report_filter.start_date.format("%Y-%m-%d").to_string();
            }
        }

        ui.add_space(10.0);

        ui.label("To:");
        // Check if current input is valid (flexible parsing)
        let end_valid = parse_flexible_date(&app.report_filter.end_date_input).is_some();
        let end_response = ui.add(
            egui::TextEdit::singleline(&mut app.report_filter.end_date_input)
                .desired_width(100.0)
                .hint_text("YYYY-MM-DD")
                .text_color(if end_valid {
                    ui.visuals().text_color()
                } else {
                    egui::Color32::from_rgb(220, 50, 50)
                }),
        );
        if end_response.changed()
            && let Some(date) = parse_flexible_date(&app.report_filter.end_date_input)
        {
            app.report_filter.end_date = date;
        }
        // On focus lost, normalize to YYYY-MM-DD format or reset if invalid
        if end_response.lost_focus() {
            if let Some(date) = parse_flexible_date(&app.report_filter.end_date_input) {
                app.report_filter.end_date = date;
                app.report_filter.end_date_input = date.format("%Y-%m-%d").to_string();
            } else {
                app.report_filter.end_date_input = app.report_filter.end_date.format("%Y-%m-%d").to_string();
            }
        }

        ui.add_space(20.0);

        // Quick date buttons (reset pagination when filter changes)
        if styled_button(ui, "Today").clicked() {
            let today = Local::now().date_naive();
            app.report_filter.start_date = today;
            app.report_filter.end_date = today;
            app.report_filter.sync_date_inputs();
            app.report_filter.reset_pagination();
        }

        if styled_button(ui, "This Week").clicked() {
            let today = Local::now().date_naive();
            let weekday = today.weekday().num_days_from_monday();
            app.report_filter.start_date = today - chrono::Duration::days(weekday as i64);
            app.report_filter.end_date = today;
            app.report_filter.sync_date_inputs();
            app.report_filter.reset_pagination();
        }

        if styled_button(ui, "This Month").clicked() {
            let today = Local::now().date_naive();
            app.report_filter.start_date = today.with_day(1).unwrap_or(today);
            app.report_filter.end_date = today;
            app.report_filter.sync_date_inputs();
            app.report_filter.reset_pagination();
        }

        if styled_button(ui, "Last 30 Days").clicked() {
            let today = Local::now().date_naive();
            app.report_filter.start_date = today - chrono::Duration::days(30);
            app.report_filter.end_date = today;
            app.report_filter.sync_date_inputs();
            app.report_filter.reset_pagination();
        }
    });

    // Format hint
    ui.label(RichText::new("Accepts: YYYY-MM-DD, YYYY/M/D, YYYY.M.D").small().weak());

    ui.add_space(10.0);

    // Department filter and generate button
    ui.horizontal(|ui| {
        ui.label("Department:");
        egui::ComboBox::from_id_salt("report_dept_filter")
            .width(200.0)
            .selected_text(
                app.report_filter
                    .department_id
                    .and_then(|id| app.departments.iter().find(|d| d.id == id))
                    .map(|d| d.name.as_str())
                    .unwrap_or("All Departments"),
            )
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.report_filter.department_id.is_none(), "All Departments")
                    .clicked()
                {
                    app.report_filter.department_id = None;
                    app.report_filter.reset_pagination();
                }
                for dept in &app.departments {
                    if ui
                        .selectable_label(app.report_filter.department_id == Some(dept.id), &dept.name)
                        .clicked()
                    {
                        app.report_filter.department_id = Some(dept.id);
                        app.report_filter.reset_pagination();
                    }
                }
            });

        ui.add_space(20.0);

        if primary_button_with_icon(ui, MAGNIFYING_GLASS, "Generate Report").clicked() {
            app.report_filter.reset_pagination();
            app.generate_report();
        }

        if styled_button_with_icon(ui, ARROWS_CLOCKWISE, "Refresh").clicked() {
            app.generate_report();
        }
    });

    ui.add_space(10.0);

    // Export buttons and record count
    ui.horizontal(|ui| {
        if styled_button_with_icon(ui, FILE_XLS, "Export Summary").clicked() {
            app.export_summary_report();
        }

        ui.add_space(10.0);

        if styled_button_with_icon(ui, FILE_XLS, "Export Detail").clicked() {
            app.export_detail_report();
        }

        ui.add_space(20.0);

        // Show total records and current page info
        let page_count = match app.report_filter.report_type {
            ReportType::Summary => app.attendance.len(),
            ReportType::Detail => app.attendance_details.len(),
        };
        let total = app.report_filter.total_records;
        let current_page = app.report_filter.current_page;
        let total_pages = app.report_filter.total_pages();
        let start_record = current_page * REPORT_PAGE_SIZE + 1;
        let end_record = (start_record - 1 + page_count as u64).min(total);

        if total > 0 {
            ui.label(format!("Showing {start_record}-{end_record} of {total} records"));
        } else {
            ui.label(format!("{page_count} records"));
        }

        // Pagination controls
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // First page
        if ui
            .add_enabled(current_page > 0, egui::Button::new(CARET_DOUBLE_LEFT))
            .on_hover_text("First page")
            .clicked()
        {
            app.first_page();
        }

        // Previous page
        if ui
            .add_enabled(current_page > 0, egui::Button::new(CARET_LEFT))
            .on_hover_text("Previous page")
            .clicked()
        {
            app.prev_page();
        }

        ui.label(format!(
            "Page {page} of {total}",
            page = current_page + 1,
            total = total_pages.max(1)
        ));

        // Next page
        if ui
            .add_enabled(current_page + 1 < total_pages, egui::Button::new(CARET_RIGHT))
            .on_hover_text("Next page")
            .clicked()
        {
            app.next_page();
        }

        // Last page
        if ui
            .add_enabled(current_page + 1 < total_pages, egui::Button::new(CARET_DOUBLE_RIGHT))
            .on_hover_text("Last page")
            .clicked()
        {
            app.last_page();
        }
    });

    ui.add_space(15.0);
    ui.separator();
    ui.add_space(10.0);

    // Results table
    match app.report_filter.report_type {
        ReportType::Summary => show_summary_table(app, ui),
        ReportType::Detail => show_detail_table(app, ui),
    }

    go_back
}

fn show_summary_table(app: &App, ui: &mut Ui) {
    ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("attendance_summary_grid")
            .num_columns(8)
            .striped(true)
            .min_col_width(80.0)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                // Header
                ui.strong("Code");
                ui.strong("Name");
                ui.strong("Department");
                ui.strong("Date");
                ui.strong("First Check");
                ui.strong("Last Check");
                ui.strong("Count");
                ui.strong("Hours");
                ui.end_row();

                // Data is already filtered at DB level via pagination
                for record in &app.attendance {
                    ui.label(&record.employee_code);
                    ui.label(&record.full_name);
                    ui.label(record.department_name.as_deref().unwrap_or("-"));
                    ui.label(record.work_date.to_string());

                    // Convert to local time for display
                    let first_local = record.first_check.with_timezone(&Local);
                    let last_local = record.last_check.with_timezone(&Local);

                    ui.label(first_local.format("%H:%M:%S").to_string());
                    ui.label(last_local.format("%H:%M:%S").to_string());
                    ui.label(record.check_count.to_string());

                    // Use pre-calculated work_hours if available
                    let hours = record.work_hours.unwrap_or_else(|| record.calculate_work_hours());
                    ui.label(format!("{:.2}", hours));

                    ui.end_row();
                }

                if app.attendance.is_empty() {
                    ui.label("No data. Click 'Generate Report' to load attendance data.");
                    ui.end_row();
                }
            });
    });
}

fn show_detail_table(app: &App, ui: &mut Ui) {
    ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("attendance_detail_grid")
            .num_columns(7)
            .striped(true)
            .min_col_width(80.0)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                // Header
                ui.strong("Code");
                ui.strong("Name");
                ui.strong("Department");
                ui.strong("Date");
                ui.strong("Time");
                ui.strong("Verify Type");
                ui.strong("Source");
                ui.end_row();

                // Data is already filtered at DB level via pagination
                for record in &app.attendance_details {
                    ui.label(record.employee_code.as_deref().unwrap_or("-"));
                    ui.label(record.full_name.as_deref().unwrap_or("-"));
                    ui.label(record.department_name.as_deref().unwrap_or("-"));

                    // Convert to local time for display
                    let check_local = record.check_time.with_timezone(&Local);

                    ui.label(check_local.format("%Y-%m-%d").to_string());
                    ui.label(check_local.format("%H:%M:%S").to_string());
                    ui.label(&record.verify_type_name);
                    ui.label(&record.source);

                    ui.end_row();
                }

                if app.attendance_details.is_empty() {
                    ui.label("No data. Click 'Generate Report' to load attendance data.");
                    ui.end_row();
                }
            });
    });
}
