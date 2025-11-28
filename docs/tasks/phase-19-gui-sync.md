# Phase 19: GUI Sync Panel

## Objective

Implement sync panel for downloading attendance from ZKTeco device.

---

## Tasks

### 19.1 Sync Panel

**`src/ui/sync_panel.rs`**

```rust
use crate::app::App;
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Sync Attendance");
    ui.separator();
    ui.add_space(10.0);

    // Device info section
    ui.group(|ui| {
        ui.heading("Device Connection");
        ui.add_space(5.0);

        egui::Grid::new("device_info_grid")
            .num_columns(2)
            .spacing([20.0, 5.0])
            .show(ui, |ui| {
                ui.label("Device URL:");
                ui.label(&app.config.device.url);
                ui.end_row();

                ui.label("Username:");
                ui.label(&app.config.device.username);
                ui.end_row();

                ui.label("Sync Days:");
                ui.label(app.config.sync.days.to_string());
                ui.end_row();

                ui.label("Max User ID:");
                ui.label(app.config.sync.max_user_id.to_string());
                ui.end_row();
            });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("Test Connection").clicked() {
                app.test_device_connection();
            }
        });
    });

    ui.add_space(20.0);

    // Sync control section
    ui.group(|ui| {
        ui.heading("Sync Control");
        ui.add_space(10.0);

        // Status
        ui.horizontal(|ui| {
            ui.label("Status:");
            let status_color = if app.is_syncing {
                egui::Color32::YELLOW
            } else if app.sync_status.contains("Failed") {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.colored_label(status_color, &app.sync_status);
        });

        ui.add_space(10.0);

        // Progress bar
        if app.is_syncing {
            ui.add(
                egui::ProgressBar::new(app.sync_progress)
                    .show_percentage()
                    .animate(true),
            );
            ui.add_space(10.0);
        }

        // Sync button
        ui.horizontal(|ui| {
            let sync_button = egui::Button::new(if app.is_syncing {
                "Syncing..."
            } else {
                "Sync Now"
            });

            if ui.add_enabled(!app.is_syncing, sync_button).clicked() {
                app.start_sync();
            }

            if app.is_syncing {
                if ui.button("Cancel").clicked() {
                    // TODO: Implement sync cancellation
                    app.log_info("Sync cancellation not implemented yet");
                }
            }
        });
    });

    ui.add_space(20.0);

    // Statistics section
    ui.group(|ui| {
        ui.heading("Statistics");
        ui.add_space(5.0);

        egui::Grid::new("sync_stats_grid")
            .num_columns(2)
            .spacing([20.0, 5.0])
            .show(ui, |ui| {
                ui.label("Total Attendance Records:");
                ui.label(app.attendance.len().to_string());
                ui.end_row();

                ui.label("Employees with Attendance:");
                let unique_employees: std::collections::HashSet<_> =
                    app.attendance.iter().map(|a| a.employee_id).collect();
                ui.label(unique_employees.len().to_string());
                ui.end_row();
            });
    });

    ui.add_space(20.0);

    // Log section
    ui.group(|ui| {
        ui.heading("Sync Log");
        ui.add_space(5.0);

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &app.log_messages {
                    let color = match entry.level {
                        crate::app::LogLevel::Info => egui::Color32::GRAY,
                        crate::app::LogLevel::Success => egui::Color32::GREEN,
                        crate::app::LogLevel::Warning => egui::Color32::YELLOW,
                        crate::app::LogLevel::Error => egui::Color32::RED,
                    };

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(
                                entry.timestamp.format("[%H:%M:%S]").to_string(),
                            )
                            .small()
                            .monospace(),
                        );
                        ui.label(egui::RichText::new(&entry.message).color(color));
                    });
                }

                if app.log_messages.is_empty() {
                    ui.label("No log entries");
                }
            });

        ui.add_space(5.0);

        if ui.button("Clear Log").clicked() {
            app.log_messages.clear();
        }
    });
}
```

### 19.2 App Methods for Sync

Add to `src/app.rs`:

```rust
impl App {
    pub fn test_device_connection(&mut self) {
        let config = self.config.clone();
        let tx = self.tx.clone();

        self.log_info("Testing device connection...");

        self.rt.spawn(async move {
            let client = crate::client::ZkClient::new(&config.device.url);
            match client.test_connection().await {
                Ok(success) => {
                    let _ = tx.send(UiMessage::DeviceTestResult(success));
                }
                Err(_) => {
                    let _ = tx.send(UiMessage::DeviceTestResult(false));
                }
            }
        });
    }

    pub fn load_attendance(&mut self) {
        self.is_loading = true;
        self.loading_message = "Loading attendance...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();
        let filter = self.report_filter.clone();

        self.rt.spawn(async move {
            match crate::db::attendance::get_daily_summary(
                &pool,
                filter.start_date,
                filter.end_date,
            )
            .await
            {
                Ok(attendance) => {
                    let _ = tx.send(UiMessage::AttendanceLoaded(attendance));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }
}
```

---

## Deliverables

- [x] Device info display
- [x] Test connection button
- [x] Sync progress bar with animation
- [x] Sync Now button
- [x] Statistics display
- [x] Scrollable log viewer
- [x] Clear log button
- [x] test_device_connection() method
- [x] load_attendance() method
