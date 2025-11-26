# Phase 21: GUI Settings Panel

## Objective

Implement settings panel for configuring device, database, and sync options.

---

## Tasks

### 21.1 Settings Panel

**`src/ui/settings.rs`**

```rust
use crate::app::App;
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Settings");
    ui.separator();
    ui.add_space(10.0);

    // Device settings
    ui.group(|ui| {
        ui.heading("Device Configuration");
        ui.add_space(5.0);

        egui::Grid::new("device_settings_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Device URL:");
                if ui
                    .text_edit_singleline(&mut app.config.device.url)
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();

                ui.label("Username:");
                if ui
                    .text_edit_singleline(&mut app.config.device.username)
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();

                ui.label("Password:");
                if ui
                    .add(egui::TextEdit::singleline(&mut app.config.device.password).password(true))
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();
            });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("Test Device Connection").clicked() {
                app.test_device_connection();
            }
        });
    });

    ui.add_space(15.0);

    // Database settings
    ui.group(|ui| {
        ui.heading("Database Configuration");
        ui.add_space(5.0);

        egui::Grid::new("db_settings_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Host:");
                if ui
                    .text_edit_singleline(&mut app.config.database.host)
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();

                ui.label("Port:");
                let mut port_str = app.config.database.port.to_string();
                if ui.text_edit_singleline(&mut port_str).changed() {
                    if let Ok(port) = port_str.parse() {
                        app.config.database.port = port;
                        app.config_modified = true;
                    }
                }
                ui.end_row();

                ui.label("Database:");
                if ui
                    .text_edit_singleline(&mut app.config.database.name)
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();

                ui.label("Username:");
                if ui
                    .text_edit_singleline(&mut app.config.database.username)
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();

                ui.label("Password:");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut app.config.database.password)
                            .password(true),
                    )
                    .changed()
                {
                    app.config_modified = true;
                }
                ui.end_row();
            });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui.button("Test Database Connection").clicked() {
                app.test_database_connection();
            }
        });
    });

    ui.add_space(15.0);

    // Sync settings
    ui.group(|ui| {
        ui.heading("Sync Options");
        ui.add_space(5.0);

        egui::Grid::new("sync_settings_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Days to sync:");
                let mut days_str = app.config.sync.days.to_string();
                if ui.text_edit_singleline(&mut days_str).changed() {
                    if let Ok(days) = days_str.parse() {
                        app.config.sync.days = days;
                        app.config_modified = true;
                    }
                }
                ui.end_row();

                ui.label("Max User ID:");
                let mut max_uid_str = app.config.sync.max_user_id.to_string();
                if ui.text_edit_singleline(&mut max_uid_str).changed() {
                    if let Ok(max_uid) = max_uid_str.parse() {
                        app.config.sync.max_user_id = max_uid;
                        app.config_modified = true;
                    }
                }
                ui.end_row();

                ui.label("Auto-sync:");
                if ui
                    .checkbox(&mut app.config.sync.auto_enabled, "Enable")
                    .changed()
                {
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
                {
                    if let Ok(interval) = interval_str.parse() {
                        app.config.sync.interval_minutes = interval;
                        app.config_modified = true;
                    }
                }
                ui.end_row();
            });
    });

    ui.add_space(15.0);

    // UI settings
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

    // Save button
    ui.horizontal(|ui| {
        let save_btn = egui::Button::new("Save Settings");
        if ui.add_enabled(app.config_modified, save_btn).clicked() {
            app.save_config();
        }

        if app.config_modified {
            ui.label(
                egui::RichText::new("(unsaved changes)")
                    .color(egui::Color32::YELLOW)
                    .italics(),
            );
        }

        if ui.button("Reset to Defaults").clicked() {
            app.config = crate::config::AppConfig::default();
            app.config_modified = true;
        }
    });
}
```

### 21.2 App Methods for Settings

Add to `src/app.rs`:

```rust
impl App {
    pub fn test_database_connection(&mut self) {
        let config = self.config.clone();
        let tx = self.tx.clone();

        self.log_info("Testing database connection...");

        self.rt.spawn(async move {
            match crate::db::create_pool(&config.database.connection_string()).await {
                Ok(pool) => match crate::db::test_connection(&pool).await {
                    Ok(_) => {
                        let _ = tx.send(UiMessage::DatabaseTestResult(true));
                    }
                    Err(_) => {
                        let _ = tx.send(UiMessage::DatabaseTestResult(false));
                    }
                },
                Err(_) => {
                    let _ = tx.send(UiMessage::DatabaseTestResult(false));
                }
            }
        });
    }

    pub fn save_config(&mut self) {
        let config_path = crate::config::AppConfig::default_path();

        match self.config.save(&config_path) {
            Ok(_) => {
                self.config_modified = false;
                self.success_message = Some("Settings saved successfully".to_string());
                self.log_success("Settings saved");
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to save settings: {}", e));
                self.log_error(format!("Failed to save settings: {}", e));
            }
        }
    }
}
```

---

## Deliverables

- [ ] Device configuration fields
- [ ] Test device connection button
- [ ] Database configuration fields
- [ ] Test database connection button
- [ ] Sync options (days, max UID, auto-sync)
- [ ] UI options (start minimized, tray)
- [ ] Save Settings button
- [ ] Reset to Defaults button
- [ ] Unsaved changes indicator
- [ ] test_database_connection() method
- [ ] save_config() method
