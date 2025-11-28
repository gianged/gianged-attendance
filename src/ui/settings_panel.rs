//! Settings panel for device, database, sync, and UI configuration.

use eframe::egui::{self, RichText};

use super::app::App;
use super::components::{back_button, colors, panel_header};

/// Show the settings panel.
/// Returns `true` if the back button was clicked.
pub fn show(app: &mut App, ui: &mut egui::Ui) -> bool {
    let go_back = back_button(ui);
    panel_header(ui, "Settings");

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Device Configuration
        ui.group(|ui| {
            ui.heading("Device Configuration");
            ui.add_space(5.0);

            egui::Grid::new("device_settings_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Device URL:");
                    if ui.text_edit_singleline(&mut app.config.device.url).changed() {
                        app.config_modified = true;
                        app.device_test_status = None; // Reset status on change
                    }
                    ui.end_row();

                    ui.label("Username:");
                    if ui.text_edit_singleline(&mut app.config.device.username).changed() {
                        app.config_modified = true;
                        app.device_test_status = None;
                    }
                    ui.end_row();

                    ui.label("Password:");
                    if ui
                        .add(egui::TextEdit::singleline(&mut app.config.device.password).password(true))
                        .changed()
                    {
                        app.config_modified = true;
                        app.device_test_status = None;
                    }
                    ui.end_row();
                });

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Test Device Connection").clicked() {
                    app.device_test_status = None;
                    app.test_device_connection();
                }

                // Inline status indicator
                match app.device_test_status {
                    Some(true) => {
                        ui.label(RichText::new("Connected").color(colors::SUCCESS));
                    }
                    Some(false) => {
                        ui.label(RichText::new("Failed").color(colors::ERROR));
                    }
                    None => {}
                }
            });
        });

        ui.add_space(15.0);

        // Database Configuration
        ui.group(|ui| {
            ui.heading("Database Configuration");
            ui.add_space(5.0);

            egui::Grid::new("db_settings_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Host:");
                    if ui.text_edit_singleline(&mut app.config.database.host).changed() {
                        app.config_modified = true;
                        app.database_test_status = None;
                    }
                    ui.end_row();

                    ui.label("Port:");
                    let mut port_str = app.config.database.port.to_string();
                    if ui.text_edit_singleline(&mut port_str).changed()
                        && let Ok(port) = port_str.parse()
                    {
                        app.config.database.port = port;
                        app.config_modified = true;
                        app.database_test_status = None;
                    }
                    ui.end_row();

                    ui.label("Database:");
                    if ui.text_edit_singleline(&mut app.config.database.name).changed() {
                        app.config_modified = true;
                        app.database_test_status = None;
                    }
                    ui.end_row();

                    ui.label("Username:");
                    if ui.text_edit_singleline(&mut app.config.database.username).changed() {
                        app.config_modified = true;
                        app.database_test_status = None;
                    }
                    ui.end_row();

                    ui.label("Password:");
                    if ui
                        .add(egui::TextEdit::singleline(&mut app.config.database.password).password(true))
                        .changed()
                    {
                        app.config_modified = true;
                        app.database_test_status = None;
                    }
                    ui.end_row();
                });

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Test Database Connection").clicked() {
                    app.database_test_status = None;
                    app.test_database_connection();
                }

                // Inline status indicator
                match app.database_test_status {
                    Some(true) => {
                        ui.label(RichText::new("Connected").color(colors::SUCCESS));
                    }
                    Some(false) => {
                        ui.label(RichText::new("Failed").color(colors::ERROR));
                    }
                    None => {}
                }
            });
        });

        ui.add_space(15.0);

        // Sync Options
        ui.group(|ui| {
            ui.heading("Sync Options");
            ui.add_space(5.0);

            egui::Grid::new("sync_settings_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Days to sync:");
                    let mut days_str = app.config.sync.days.to_string();
                    if ui.text_edit_singleline(&mut days_str).changed()
                        && let Ok(days) = days_str.parse()
                    {
                        app.config.sync.days = days;
                        app.config_modified = true;
                    }
                    ui.end_row();

                    ui.label("Max User ID:");
                    let mut max_uid_str = app.config.sync.max_user_id.to_string();
                    if ui.text_edit_singleline(&mut max_uid_str).changed()
                        && let Ok(max_uid) = max_uid_str.parse()
                    {
                        app.config.sync.max_user_id = max_uid;
                        app.config_modified = true;
                    }
                    ui.end_row();

                    ui.label("Auto-sync:");
                    if ui.checkbox(&mut app.config.sync.auto_enabled, "Enable").changed() {
                        app.config_modified = true;
                    }
                    ui.end_row();

                    ui.label("Interval (minutes):");
                    let mut interval_str = app.config.sync.interval_minutes.to_string();
                    if ui
                        .add_enabled(
                            app.config.sync.auto_enabled,
                            egui::TextEdit::singleline(&mut interval_str),
                        )
                        .changed()
                        && let Ok(interval) = interval_str.parse()
                    {
                        app.config.sync.interval_minutes = interval;
                        app.config_modified = true;
                    }
                    ui.end_row();
                });
        });

        ui.add_space(15.0);

        // UI Options
        ui.group(|ui| {
            ui.heading("UI Options");
            ui.add_space(5.0);

            if ui
                .checkbox(&mut app.config.ui.start_minimized, "Start minimized")
                .changed()
            {
                app.config_modified = true;
            }

            if ui
                .checkbox(&mut app.config.ui.minimize_to_tray, "Minimize to system tray")
                .changed()
            {
                app.config_modified = true;
            }
        });

        ui.add_space(20.0);

        // Action buttons
        ui.horizontal(|ui| {
            let save_btn = egui::Button::new("Save Settings");
            if ui.add_enabled(app.config_modified, save_btn).clicked() {
                app.save_config();
            }

            if app.config_modified {
                ui.label(RichText::new("(unsaved changes)").color(colors::WARNING).italics());
            }

            if ui.button("Reset to Defaults").clicked() {
                app.config = crate::config::AppConfig::default();
                app.config_modified = true;
                app.device_test_status = None;
                app.database_test_status = None;
            }
        });
    });

    go_back
}
