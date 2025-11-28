//! Sync panel for device synchronization with device info, statistics, and log viewer.

use std::collections::HashSet;

use eframe::egui::{self, Color32, ProgressBar, RichText, ScrollArea, Ui};

use super::app::{App, LogLevel, SyncState};
use super::components::{back_button, colors, panel_header, styled_button_with_icon};
use egui_phosphor::regular::{ARROWS_CLOCKWISE, PLUGS_CONNECTED, TRASH};

/// Show the sync panel.
///
/// Returns `true` if the back button was clicked.
pub fn show(app: &mut App, ui: &mut Ui) -> bool {
    let mut go_back = false;

    if back_button(ui) {
        go_back = true;
    }

    panel_header(ui, "Device Sync");

    // Device Info Section
    show_device_info(app, ui);

    ui.add_space(20.0);

    // Sync Control Section
    show_sync_control(app, ui);

    ui.add_space(20.0);

    // Statistics Section
    show_statistics(app, ui);

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
                        ui.label(format!("Syncing: {}", message));
                    });
                    ui.add_space(10.0);
                    ui.add(ProgressBar::new(*progress).show_percentage().animate(true));
                }
                SyncState::Completed { records_synced } => {
                    ui.colored_label(colors::SUCCESS, format!("Completed: {} records synced", records_synced));
                }
                SyncState::Error(err) => {
                    ui.colored_label(colors::ERROR, format!("Error: {}", err));
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
                        egui::Button::new(RichText::new(format!("{} Sync Now", ARROWS_CLOCKWISE))),
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

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(entry.timestamp.format("[%H:%M:%S]").to_string())
                                        .small()
                                        .monospace()
                                        .color(Color32::DARK_GRAY),
                                );
                                ui.label(RichText::new(&entry.message).color(color));
                            });
                        }
                    }
                });
        });
}
