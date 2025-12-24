//! Sync panel for device synchronization with device info, statistics, and log viewer.

use std::collections::HashSet;

use eframe::egui::{self, Color32, ProgressBar, RichText, ScrollArea, Ui};

use super::app::{App, LogLevel, SyncState};
use super::components::{back_button, colors, panel_header, styled_button_with_icon};
use egui_phosphor::regular::{ARROWS_CLOCKWISE, DATABASE, PLUGS_CONNECTED, TRASH, WARNING};

/// Show the sync panel.
///
/// Returns `true` if the back button was clicked.
pub fn show(app: &mut App, ui: &mut Ui) -> bool {
    let mut go_back = false;

    if back_button(ui) {
        go_back = true;
    }

    panel_header(ui, "Device Sync");

    // Top row: Device Info + Statistics side by side
    ui.columns(2, |columns| {
        // Left column: Device Info
        show_device_info(app, &mut columns[0]);

        // Right column: Statistics
        show_statistics(app, &mut columns[1]);
    });

    ui.add_space(20.0);

    // Middle row: Sync Control + Device Capacity side by side
    ui.columns(2, |columns| {
        // Left column: Sync Control
        show_sync_control(app, &mut columns[0]);

        // Right column: Device Capacity
        show_device_capacity(app, &mut columns[1]);
    });

    ui.add_space(20.0);

    // Log Viewer Section
    show_log_viewer(app, ui);

    go_back
}

fn show_device_info(app: &mut App, ui: &mut Ui) {
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(egui::Margin::same(15))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new("Device Connection").strong());
            ui.add_space(10.0);

            egui::Grid::new("device_info_grid")
                .num_columns(2)
                .spacing([20.0, 6.0])
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

            if styled_button_with_icon(ui, PLUGS_CONNECTED, "Test Connection").clicked() {
                app.test_device_connection();
            }
        });
}

fn show_device_capacity(app: &mut App, ui: &mut Ui) {
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(egui::Margin::same(15))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new("Device Storage").strong());
            ui.add_space(10.0);

            // Capacity display
            if let Some(capacity) = &app.device_capacity {
                let usage_percent = if capacity.records_cap > 0 {
                    capacity.records as f32 / capacity.records_cap as f32
                } else {
                    0.0
                };

                egui::Grid::new("capacity_grid")
                    .num_columns(2)
                    .spacing([20.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Records:");
                        ui.label(format!("{} / {}", capacity.records, capacity.records_cap));
                        ui.end_row();

                        ui.label("Available:");
                        ui.label(capacity.records_av.to_string());
                        ui.end_row();

                        ui.label("Usage:");
                        let bar_color = if usage_percent > 0.8 {
                            colors::ERROR
                        } else if usage_percent > 0.6 {
                            colors::WARNING
                        } else {
                            colors::SUCCESS
                        };
                        ui.add(ProgressBar::new(usage_percent).fill(bar_color).show_percentage());
                        ui.end_row();
                    });

                // Warning banner when usage is high
                if usage_percent > 0.75 {
                    ui.add_space(8.0);
                    let (bg_color, text_color, message) = if usage_percent > 0.9 {
                        (
                            Color32::from_rgb(254, 226, 226),
                            Color32::from_rgb(153, 27, 27),
                            "Critical: Device storage almost full!",
                        )
                    } else {
                        (
                            Color32::from_rgb(254, 249, 195),
                            Color32::from_rgb(133, 77, 14),
                            "Warning: Consider clearing old records",
                        )
                    };

                    egui::Frame::new()
                        .fill(bg_color)
                        .inner_margin(egui::Margin::same(8))
                        .corner_radius(egui::CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(WARNING).color(text_color));
                                ui.label(RichText::new(message).color(text_color));
                            });
                        });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Auto-clear section with status indicator
                ui.label(RichText::new("Auto-Clear").strong());
                ui.add_space(4.0);

                egui::Frame::new()
                    .fill(ui.style().visuals.faint_bg_color)
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Status dot indicator
                            let indicator_color = if app.config.sync.auto_clear_enabled {
                                Color32::from_rgb(34, 197, 94) // Green
                            } else {
                                Color32::from_rgb(156, 163, 175) // Gray
                            };
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 5.0, indicator_color);

                            let status_text = if app.config.sync.auto_clear_enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            };
                            ui.label(RichText::new(status_text).strong());

                            if app.config.sync.auto_clear_enabled {
                                ui.label("|");
                                ui.label(format!("Threshold: {} records", app.config.sync.auto_clear_threshold));
                            }
                        });

                        // Show warning if threshold exceeded
                        if app.config.sync.auto_clear_enabled
                            && capacity.records >= app.config.sync.auto_clear_threshold
                        {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Will clear on next sync")
                                    .small()
                                    .color(colors::WARNING),
                            );
                        }
                    });

                ui.add_space(10.0);

                // Manual clear section
                ui.label(RichText::new("Manual Clear").strong());
                ui.add_space(4.0);
            } else if app.device_capacity_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading capacity...");
                });
            } else {
                ui.label(RichText::new("Capacity not loaded").weak());
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let can_refresh = !app.device_capacity_loading && !app.device_clearing;
                if ui
                    .add_enabled(
                        can_refresh,
                        egui::Button::new(RichText::new(format!("{DATABASE} Refresh"))),
                    )
                    .clicked()
                {
                    app.fetch_device_capacity();
                }

                let can_clear = !app.device_capacity_loading && !app.device_clearing && app.device_capacity.is_some();
                if ui
                    .add_enabled(
                        can_clear,
                        egui::Button::new(RichText::new(format!("{TRASH} Clear Device"))),
                    )
                    .clicked()
                {
                    app.show_clear_confirm = true;
                }

                if app.device_clearing {
                    ui.spinner();
                    ui.label("Clearing...");
                }
            });
        });
}

fn show_sync_control(app: &mut App, ui: &mut Ui) {
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(egui::Margin::same(15))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new("Sync Control").strong());
            ui.add_space(10.0);

            // Last sync time
            ui.horizontal(|ui| {
                ui.label("Last sync:");
                if let Some(time) = app.last_sync_time {
                    ui.label(time.format("%Y-%m-%d %H:%M:%S").to_string());
                } else {
                    ui.label(RichText::new("Never").weak());
                }
            });

            ui.add_space(10.0);

            // Status indicator
            match &app.sync_state {
                SyncState::Idle => {
                    ui.colored_label(colors::NEUTRAL, "Status: Idle");
                }
                SyncState::InProgress { progress, message } => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("Syncing: {message}"));
                    });
                    ui.add_space(10.0);
                    ui.add(ProgressBar::new(*progress).show_percentage().animate(true));
                }
                SyncState::Completed { records_synced } => {
                    ui.colored_label(colors::SUCCESS, format!("Completed: {records_synced} records synced"));
                }
                SyncState::Error(err) => {
                    ui.colored_label(colors::ERROR, format!("Error: {err}"));
                }
            }

            ui.add_space(15.0);

            // Sync button
            let can_sync = matches!(
                app.sync_state,
                SyncState::Idle | SyncState::Completed { .. } | SyncState::Error(_)
            );

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        can_sync,
                        egui::Button::new(RichText::new(format!("{ARROWS_CLOCKWISE} Sync Now"))),
                    )
                    .clicked()
                {
                    app.start_sync();
                }
            });
        });
}

fn show_statistics(app: &App, ui: &mut Ui) {
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(egui::Margin::same(15))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new("Statistics").strong());
            ui.add_space(10.0);

            egui::Grid::new("sync_stats_grid")
                .num_columns(2)
                .spacing([20.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Total Attendance Records:");
                    ui.label(app.attendance.len().to_string());
                    ui.end_row();

                    ui.label("Employees with Attendance:");
                    let unique_employees: HashSet<_> = app.attendance.iter().map(|a| a.employee_id).collect();
                    ui.label(unique_employees.len().to_string());
                    ui.end_row();

                    ui.label("Total Employees:");
                    ui.label(app.employees.len().to_string());
                    ui.end_row();

                    ui.label("Total Departments:");
                    ui.label(app.departments.len().to_string());
                    ui.end_row();
                });
        });
}

fn show_log_viewer(app: &mut App, ui: &mut Ui) {
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(egui::Margin::same(15))
        .corner_radius(egui::CornerRadius::same(8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Sync Log").strong());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if styled_button_with_icon(ui, TRASH, "Clear").clicked() {
                        app.clear_log();
                    }
                });
            });

            ui.add_space(10.0);

            ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    // Constrain width to enable text wrapping
                    ui.set_width(ui.available_width());

                    if app.log_messages.is_empty() {
                        ui.label(RichText::new("No log entries").weak());
                    } else {
                        for entry in &app.log_messages {
                            let color = match entry.level {
                                LogLevel::Info => Color32::GRAY,
                                LogLevel::Success => colors::SUCCESS,
                                LogLevel::Warning => colors::WARNING,
                                LogLevel::Error => colors::ERROR,
                            };

                            // Format as single line with wrapped text
                            let formatted = format!(
                                "[{}] {}",
                                entry.timestamp.format("%H:%M:%S"),
                                entry.message
                            );
                            ui.add(egui::Label::new(RichText::new(formatted).color(color)).wrap());
                        }
                    }
                });
        });
}
