//! Dashboard panel with stats, navigation cards, quick actions, and activity log.

use std::collections::HashSet;

use chrono::Local;
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, ScrollArea, Ui};
use egui_phosphor::regular::{ARROWS_CLOCKWISE, BUILDINGS, CHART_BAR, USERS};

use super::app::{App, LogLevel, Panel, SyncState};
use super::components::dashboard_card;

/// Show the dashboard panel.
///
/// Returns `Some(panel)` if navigation is requested.
pub fn show(app: &mut App, ui: &mut Ui) -> Option<Panel> {
    let mut next_panel = None;

    ui.vertical_centered(|ui| {
        ui.add_space(30.0);

        // Header
        ui.label(RichText::new("Gianged Attendance").size(32.0).strong());
        ui.add_space(5.0);
        ui.label(RichText::new("Staff and Attendance Management").size(14.0).weak());

        ui.add_space(30.0);

        // Stat cards row
        ui.horizontal(|ui| {
            let available = ui.available_width();
            let start_offset = ((available - 510.0) / 2.0).max(0.0);
            ui.add_space(start_offset);

            stat_card(
                ui,
                "Total Employees",
                &app.employees.len().to_string(),
                "Active staff members",
            );
            stat_card(
                ui,
                "Departments",
                &app.departments.len().to_string(),
                "Active departments",
            );
            stat_card(
                ui,
                "Today's Attendance",
                &count_today_attendance(app).to_string(),
                "Employees checked in",
            );
        });

        ui.add_space(30.0);

        // Navigation cards row
        let available = ui.available_width();
        let num_cards = 4.0;
        let spacing = 30.0;
        let total_spacing = spacing * (num_cards - 1.0);
        let card_width = ((available - total_spacing) / num_cards).clamp(150.0, 250.0);
        let card_height = card_width * 0.75;
        let card_size = egui::vec2(card_width, card_height);
        let total_width = card_width * num_cards + total_spacing;
        let start_offset = ((available - total_width) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            ui.add_space(start_offset);

            if dashboard_card(ui, "Manage Departments", "Organize staff groups", BUILDINGS, card_size).clicked() {
                next_panel = Some(Panel::Departments);
            }

            ui.add_space(spacing);

            if dashboard_card(ui, "Manage Staff", "Employee records", USERS, card_size).clicked() {
                next_panel = Some(Panel::Employees);
            }

            ui.add_space(spacing);

            if dashboard_card(ui, "Device Sync", "Sync attendance data", ARROWS_CLOCKWISE, card_size).clicked() {
                next_panel = Some(Panel::Sync);
            }

            ui.add_space(spacing);

            if dashboard_card(ui, "Reports", "Attendance reports & export", CHART_BAR, card_size).clicked() {
                next_panel = Some(Panel::Reports);
            }
        });

        ui.add_space(30.0);
    });

    // Two-column layout: Quick Actions | Recent Activity
    let available_width = ui.available_width();
    let column_width = (available_width - 40.0) / 2.0;

    ui.horizontal(|ui| {
        ui.add_space(10.0);

        // Left column - Quick Actions
        ui.vertical(|ui| {
            ui.set_width(column_width);

            egui::Frame::new()
                .fill(ui.style().visuals.extreme_bg_color)
                .inner_margin(Margin::same(15))
                .corner_radius(CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(column_width - 30.0);

                    ui.label(RichText::new("Quick Actions").strong());
                    ui.add_space(10.0);

                    let is_syncing = matches!(app.sync_state, SyncState::InProgress { .. });

                    ui.add_enabled_ui(!is_syncing, |ui| {
                        if ui.button("Sync Now").clicked() {
                            app.start_sync();
                        }
                    });

                    ui.add_space(5.0);

                    if ui.button("Export Today's Report").clicked() {
                        app.export_today_report();
                    }

                    ui.add_space(5.0);

                    if ui.button("Add Employee").clicked() {
                        app.employee_form.reset();
                        app.employee_form.is_open = true;
                        next_panel = Some(Panel::Employees);
                    }
                });
        });

        ui.add_space(20.0);

        // Right column - Recent Activity
        ui.vertical(|ui| {
            ui.set_width(column_width);

            egui::Frame::new()
                .fill(ui.style().visuals.extreme_bg_color)
                .inner_margin(Margin::same(15))
                .corner_radius(CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(column_width - 30.0);

                    ui.label(RichText::new("Recent Activity").strong());
                    ui.add_space(10.0);

                    ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        if app.log_messages.is_empty() {
                            ui.label(RichText::new("No recent activity").weak());
                        } else {
                            for entry in app.log_messages.iter().rev().take(10) {
                                let color = match entry.level {
                                    LogLevel::Info => Color32::GRAY,
                                    LogLevel::Success => Color32::from_rgb(100, 200, 100),
                                    LogLevel::Warning => Color32::from_rgb(230, 180, 50),
                                    LogLevel::Error => Color32::from_rgb(230, 100, 100),
                                };

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(entry.timestamp.format("%H:%M:%S").to_string())
                                            .small()
                                            .color(Color32::DARK_GRAY),
                                    );
                                    ui.label(RichText::new(&entry.message).color(color));
                                });
                            }
                        }
                    });
                });
        });
    });

    ui.add_space(20.0);

    // Sync Status Section
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(Margin::same(15))
        .outer_margin(Margin::symmetric(10, 0))
        .corner_radius(CornerRadius::same(8))
        .show(ui, |ui| {
            ui.label(RichText::new("Sync Status").strong());
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Status:");
                match &app.sync_state {
                    SyncState::Idle => ui.label("Idle"),
                    SyncState::InProgress { message, .. } => {
                        ui.label(RichText::new(message).color(Color32::from_rgb(100, 150, 230)))
                    }
                    SyncState::Completed { records_synced } => ui.label(
                        RichText::new(format!("Completed ({records_synced} records)"))
                            .color(Color32::from_rgb(100, 200, 100)),
                    ),
                    SyncState::Error(e) => ui.label(RichText::new(e).color(Color32::from_rgb(230, 100, 100))),
                };
            });

            if let SyncState::InProgress { progress, .. } = &app.sync_state {
                ui.add_space(5.0);
                ui.add(egui::ProgressBar::new(*progress).show_percentage());
            }

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Device:");
                ui.label(RichText::new(&app.config.device.url).weak());
            });

            if let Some(last_sync) = app.last_sync_time {
                ui.horizontal(|ui| {
                    ui.label("Last sync:");
                    ui.label(RichText::new(last_sync.format("%Y-%m-%d %H:%M:%S").to_string()).weak());
                });
            }
        });

    next_panel
}

/// Render a stat card with title, value, and subtitle.
fn stat_card(ui: &mut Ui, title: &str, value: &str, subtitle: &str) {
    egui::Frame::new()
        .fill(ui.style().visuals.extreme_bg_color)
        .inner_margin(Margin::same(15))
        .outer_margin(Margin::same(5))
        .corner_radius(CornerRadius::same(8))
        .show(ui, |ui| {
            ui.set_min_width(150.0);

            ui.vertical(|ui| {
                ui.label(RichText::new(title).small());
                ui.label(RichText::new(value).heading().strong());
                ui.label(RichText::new(subtitle).small().weak());
            });
        });
}

/// Count unique employees who checked in today.
fn count_today_attendance(app: &App) -> usize {
    let today = Local::now().date_naive();
    app.attendance
        .iter()
        .filter(|a| a.work_date == today)
        .map(|a| a.employee_id)
        .collect::<HashSet<_>>()
        .len()
}
