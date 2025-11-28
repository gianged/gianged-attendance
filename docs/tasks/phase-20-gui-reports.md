# Phase 20: GUI Reports Panel

## Objective

Implement reports panel with filters and Excel export.

---

## Tasks

### 20.1 Reports Panel

**`src/ui/reports.rs`**

```rust
use crate::app::App;
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Attendance Reports");
    ui.separator();
    ui.add_space(10.0);

    // Filter section
    ui.group(|ui| {
        ui.heading("Filters");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            // Date range
            ui.label("From:");
            let mut start_str = app.report_filter.start_date.to_string();
            if ui.text_edit_singleline(&mut start_str).changed() {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&start_str, "%Y-%m-%d") {
                    app.report_filter.start_date = date;
                }
            }

            ui.label("To:");
            let mut end_str = app.report_filter.end_date.to_string();
            if ui.text_edit_singleline(&mut end_str).changed() {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(&end_str, "%Y-%m-%d") {
                    app.report_filter.end_date = date;
                }
            }

            ui.separator();

            // Quick date buttons
            if ui.button("Today").clicked() {
                let today = chrono::Local::now().date_naive();
                app.report_filter.start_date = today;
                app.report_filter.end_date = today;
            }
            if ui.button("This Week").clicked() {
                let today = chrono::Local::now().date_naive();
                let weekday = today.weekday().num_days_from_monday();
                app.report_filter.start_date = today - chrono::Duration::days(weekday as i64);
                app.report_filter.end_date = today;
            }
            if ui.button("This Month").clicked() {
                let today = chrono::Local::now().date_naive();
                app.report_filter.start_date = today.with_day(1).unwrap_or(today);
                app.report_filter.end_date = today;
            }
            if ui.button("Last 30 Days").clicked() {
                let today = chrono::Local::now().date_naive();
                app.report_filter.start_date = today - chrono::Duration::days(30);
                app.report_filter.end_date = today;
            }
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            // Department filter
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
                        .selectable_label(
                            app.report_filter.department_id.is_none(),
                            "All Departments",
                        )
                        .clicked()
                    {
                        app.report_filter.department_id = None;
                    }
                    for dept in &app.departments {
                        if ui
                            .selectable_label(
                                app.report_filter.department_id == Some(dept.id),
                                &dept.name,
                            )
                            .clicked()
                        {
                            app.report_filter.department_id = Some(dept.id);
                        }
                    }
                });

            ui.separator();

            if ui.button("Generate Report").clicked() {
                app.generate_report();
            }
        });
    });

    ui.add_space(10.0);

    // Actions bar
    ui.horizontal(|ui| {
        if ui.button("Export to Excel").clicked() {
            app.export_report();
        }

        ui.separator();

        ui.label(format!("{} records", app.attendance.len()));
    });

    ui.add_space(10.0);

    // Results table
    show_results_table(app, ui);
}

fn show_results_table(app: &App, ui: &mut egui::Ui) {
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("attendance_report_grid")
            .num_columns(8)
            .striped(true)
            .min_col_width(80.0)
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

                // Filter by department if selected
                let filtered: Vec<_> = app
                    .attendance
                    .iter()
                    .filter(|a| {
                        app.report_filter.department_id.is_none()
                            || app
                                .employees
                                .iter()
                                .find(|e| e.id == a.employee_id)
                                .and_then(|e| e.department_id)
                                == app.report_filter.department_id
                    })
                    .collect();

                for record in filtered {
                    ui.label(&record.employee_code);
                    ui.label(&record.full_name);
                    ui.label(record.department_name.as_deref().unwrap_or("-"));
                    ui.label(record.work_date.to_string());
                    ui.label(record.first_check.format("%H:%M:%S").to_string());
                    ui.label(record.last_check.format("%H:%M:%S").to_string());
                    ui.label(record.check_count.to_string());

                    // Calculate hours
                    let hours = record.work_hours();
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
```

### 20.2 App Methods for Reports

Add to `src/app.rs`:

```rust
impl App {
    pub fn generate_report(&mut self) {
        self.is_loading = true;
        self.loading_message = "Generating report...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();
        let filter = self.report_filter.clone();

        self.rt.spawn(async move {
            let result = if let Some(dept_id) = filter.department_id {
                crate::db::attendance::get_daily_summary_by_department(
                    &pool,
                    dept_id,
                    filter.start_date,
                    filter.end_date,
                )
                .await
            } else {
                crate::db::attendance::get_daily_summary(
                    &pool,
                    filter.start_date,
                    filter.end_date,
                )
                .await
            };

            match result {
                Ok(attendance) => {
                    let _ = tx.send(UiMessage::AttendanceLoaded(attendance));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    pub fn export_report(&mut self) {
        if self.attendance.is_empty() {
            self.error_message = Some("No data to export. Generate a report first.".to_string());
            return;
        }

        let filename = crate::export::generate_export_filename("attendance_report");
        let path = std::path::PathBuf::from(&filename);

        match crate::export::export_attendance_to_excel(&self.attendance, &path) {
            Ok(_) => {
                let _ = self.tx.send(UiMessage::ExportCompleted(filename));
            }
            Err(e) => {
                let _ = self.tx.send(UiMessage::ExportFailed(e.to_string()));
            }
        }
    }
}
```

---

## Deliverables

- [x] Date range filter (from/to)
- [x] Quick date buttons (Today, This Week, This Month, Last 30 Days)
- [x] Department filter dropdown
- [x] Generate Report button
- [x] Export to Excel button
- [x] Results table with all columns
- [x] Work hours calculation
- [x] generate_report() method
- [x] export_report() method
