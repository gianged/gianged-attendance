# Phase 16: GUI Dashboard Panel

## Objective

Implement dashboard panel with quick stats and recent activity.

---

## Tasks

### 16.1 Dashboard Component

**`src/ui/dashboard.rs`**

```rust
use crate::app::App;
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Dashboard");
    ui.separator();
    ui.add_space(10.0);

    // Stats cards row
    ui.horizontal(|ui| {
        stat_card(ui, "Total Employees", &app.employees.len().to_string(), "Active staff members");
        stat_card(ui, "Departments", &app.departments.len().to_string(), "Active departments");
        stat_card(ui, "Today's Attendance", &count_today_attendance(app).to_string(), "Employees checked in");
    });

    ui.add_space(20.0);

    // Two column layout
    ui.columns(2, |columns| {
        // Left column - Quick actions
        columns[0].group(|ui| {
            ui.heading("Quick Actions");
            ui.add_space(10.0);

            if ui.button("Sync Now").clicked() {
                app.start_sync();
            }

            ui.add_space(5.0);

            if ui.button("Export Today's Report").clicked() {
                app.export_today_report();
            }

            ui.add_space(5.0);

            if ui.button("Add Employee").clicked() {
                app.employee_form.reset();
                app.employee_form.is_open = true;
                app.current_panel = crate::app::Panel::Employees;
            }
        });

        // Right column - Recent activity
        columns[1].group(|ui| {
            ui.heading("Recent Activity");
            ui.add_space(10.0);

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for entry in app.log_messages.iter().rev().take(10) {
                        let color = match entry.level {
                            crate::app::LogLevel::Info => egui::Color32::GRAY,
                            crate::app::LogLevel::Success => egui::Color32::GREEN,
                            crate::app::LogLevel::Warning => egui::Color32::YELLOW,
                            crate::app::LogLevel::Error => egui::Color32::RED,
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(entry.timestamp.format("%H:%M:%S").to_string())
                                    .small()
                                    .color(egui::Color32::DARK_GRAY),
                            );
                            ui.label(egui::RichText::new(&entry.message).color(color));
                        });
                    }

                    if app.log_messages.is_empty() {
                        ui.label("No recent activity");
                    }
                });
        });
    });

    ui.add_space(20.0);

    // Sync status section
    ui.group(|ui| {
        ui.heading("Sync Status");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Status:");
            ui.label(&app.sync_status);
        });

        if app.is_syncing {
            ui.add(egui::ProgressBar::new(app.sync_progress).show_percentage());
        }

        ui.horizontal(|ui| {
            ui.label("Device:");
            ui.label(&app.config.device.url);
        });
    });
}

fn stat_card(ui: &mut egui::Ui, title: &str, value: &str, subtitle: &str) {
    egui::Frame::none()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(egui::Margin::same(15.0))
        .outer_margin(egui::Margin::same(5.0))
        .rounding(egui::Rounding::same(8.0))
        .show(ui, |ui| {
            ui.set_min_width(150.0);

            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).small());
                ui.label(egui::RichText::new(value).heading().strong());
                ui.label(egui::RichText::new(subtitle).small().weak());
            });
        });
}

fn count_today_attendance(app: &App) -> usize {
    let today = chrono::Local::now().date_naive();
    app.attendance
        .iter()
        .filter(|a| a.work_date == today)
        .map(|a| a.employee_id)
        .collect::<std::collections::HashSet<_>>()
        .len()
}
```

### 16.2 App Helper Methods

Add to `src/app.rs`:

```rust
impl App {
    pub fn start_sync(&mut self) {
        if self.is_syncing {
            return;
        }

        self.is_syncing = true;
        self.sync_progress = 0.0;
        self.sync_status = "Starting sync...".to_string();

        let config = self.config.clone();
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            crate::sync::run_sync_background(config, (*pool).clone(), tx).await;
        });
    }

    pub fn export_today_report(&mut self) {
        let today = chrono::Local::now().date_naive();
        let data: Vec<_> = self
            .attendance
            .iter()
            .filter(|a| a.work_date == today)
            .cloned()
            .collect();

        if data.is_empty() {
            self.error_message = Some("No attendance data for today".to_string());
            return;
        }

        let filename = crate::export::generate_export_filename("attendance_today");
        let path = std::path::PathBuf::from(&filename);

        match crate::export::export_attendance_to_excel(&data, &path) {
            Ok(_) => {
                self.success_message = Some(format!("Exported to: {}", filename));
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
            }
        }
    }
}
```

---

## Deliverables

- [x] Dashboard layout
- [x] Stat cards (employees, departments, today's attendance)
- [x] Quick actions section
- [x] Recent activity log
- [x] Sync status display
- [x] start_sync() method
- [x] export_today_report() method
