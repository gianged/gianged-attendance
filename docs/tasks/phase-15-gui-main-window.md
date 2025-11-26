# Phase 15: GUI Main Window and Sidebar

## Objective

Implement main window layout and sidebar navigation.

---

## Tasks

### 15.1 Main Entry Point

**`src/main.rs`**

```rust
mod app;
mod client;
mod config;
mod db;
mod error;
mod export;
mod models;
mod sync;
mod ui;

use app::App;
use config::AppConfig;
use eframe::egui;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Load or create config
    let config_path = AppConfig::default_path();
    let config = if config_path.exists() {
        AppConfig::load(&config_path).unwrap_or_default()
    } else {
        let config = AppConfig::default();
        let _ = config.save(&config_path);
        config
    };

    // Create tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime");

    // Create database pool
    let pool = rt.block_on(async {
        db::create_pool(&config.database.connection_string())
            .await
            .expect("Failed to connect to database")
    });

    // Window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("GiangEd Attendance")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    // Run application
    eframe::run_native(
        "GiangEd Attendance",
        options,
        Box::new(|cc| {
            // Set default fonts/style if needed
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(Arc::new(pool), config, rt)))
        }),
    )
}
```

### 15.2 eframe App Implementation

**`src/app.rs`** (add impl eframe::App):

```rust
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process messages from async tasks
        self.process_messages();

        // Top panel (menu bar)
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui::menu_bar::show(self, ui);
        });

        // Bottom panel (status bar)
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui::status_bar::show(self, ui);
        });

        // Left panel (sidebar)
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .default_width(180.0)
            .show(ctx, |ui| {
                ui::sidebar::show(self, ui);
            });

        // Central panel (main content)
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_panel {
                Panel::Dashboard => ui::dashboard::show(self, ui),
                Panel::Departments => ui::departments::show(self, ui),
                Panel::Employees => ui::employees::show(self, ui),
                Panel::Sync => ui::sync_panel::show(self, ui),
                Panel::Reports => ui::reports::show(self, ui),
                Panel::Settings => ui::settings::show(self, ui),
            }
        });

        // Modal dialogs
        self.show_dialogs(ctx);

        // Request repaint if syncing (for progress updates)
        if self.is_syncing || self.is_loading {
            ctx.request_repaint();
        }
    }
}

impl App {
    fn process_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UiMessage::DepartmentsLoaded(depts) => {
                    self.departments = depts;
                    self.is_loading = false;
                }
                UiMessage::EmployeesLoaded(emps) => {
                    self.employees = emps;
                    self.is_loading = false;
                }
                UiMessage::AttendanceLoaded(att) => {
                    self.attendance = att;
                    self.is_loading = false;
                }
                UiMessage::LoadError(err) => {
                    self.error_message = Some(err.clone());
                    self.log_error(err);
                    self.is_loading = false;
                }
                UiMessage::SyncProgress(progress, message) => {
                    self.sync_progress = progress;
                    self.sync_status = message.clone();
                    self.log_info(message);
                }
                UiMessage::SyncCompleted(result) => {
                    self.is_syncing = false;
                    self.sync_status = result.summary();
                    self.log_success(format!("Sync completed: {}", result.summary()));
                    self.load_attendance();
                }
                UiMessage::SyncFailed(err) => {
                    self.is_syncing = false;
                    self.sync_status = format!("Failed: {}", err);
                    self.error_message = Some(err.clone());
                    self.log_error(err);
                }
                UiMessage::DepartmentSaved(dept) => {
                    self.log_success(format!("Department saved: {}", dept.name));
                    self.department_form.reset();
                    self.load_departments();
                }
                UiMessage::DepartmentDeleted(id) => {
                    self.log_success(format!("Department deleted: {}", id));
                    self.load_departments();
                }
                UiMessage::EmployeeSaved(emp) => {
                    self.log_success(format!("Employee saved: {}", emp.full_name));
                    self.employee_form.reset();
                    self.load_employees();
                }
                UiMessage::EmployeeDeleted(id) => {
                    self.log_success(format!("Employee deleted: {}", id));
                    self.load_employees();
                }
                UiMessage::OperationFailed(err) => {
                    self.error_message = Some(err.clone());
                    self.log_error(err);
                }
                UiMessage::ExportCompleted(path) => {
                    self.success_message = Some(format!("Exported to: {}", path));
                    self.log_success(format!("Exported to: {}", path));
                }
                UiMessage::ExportFailed(err) => {
                    self.error_message = Some(err.clone());
                    self.log_error(err);
                }
                UiMessage::DeviceTestResult(success) => {
                    if success {
                        self.success_message = Some("Device connection successful!".to_string());
                    } else {
                        self.error_message = Some("Device connection failed!".to_string());
                    }
                }
                UiMessage::DatabaseTestResult(success) => {
                    if success {
                        self.success_message = Some("Database connection successful!".to_string());
                    } else {
                        self.error_message = Some("Database connection failed!".to_string());
                    }
                }
            }
        }
    }

    fn show_dialogs(&mut self, ctx: &egui::Context) {
        // Error dialog
        if let Some(ref error) = self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error);
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }

        // Success dialog
        if let Some(ref msg) = self.success_message.clone() {
            egui::Window::new("Success")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(msg);
                    if ui.button("OK").clicked() {
                        self.success_message = None;
                    }
                });
        }

        // Delete confirmation
        if self.show_delete_confirm {
            if let Some(ref target) = self.delete_target.clone() {
                let (title, message) = match target {
                    DeleteTarget::Department(_, name) => {
                        ("Delete Department", format!("Delete department '{}'?", name))
                    }
                    DeleteTarget::Employee(_, name) => {
                        ("Delete Employee", format!("Delete employee '{}'?", name))
                    }
                };

                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label(message);
                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_delete_confirm = false;
                                self.delete_target = None;
                            }
                            if ui.button("Delete").clicked() {
                                self.confirm_delete();
                                self.show_delete_confirm = false;
                                self.delete_target = None;
                            }
                        });
                    });
            }
        }
    }
}
```

### 15.3 Sidebar Component

**`src/ui/sidebar.rs`**

```rust
use crate::app::{App, Panel};
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.add_space(10.0);

        // App title
        ui.heading("GiangEd");
        ui.label("Attendance");

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Navigation buttons
        nav_button(ui, app, Panel::Dashboard, "Dashboard");
        nav_button(ui, app, Panel::Departments, "Departments");
        nav_button(ui, app, Panel::Employees, "Employees");
        nav_button(ui, app, Panel::Sync, "Sync");
        nav_button(ui, app, Panel::Reports, "Reports");

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        nav_button(ui, app, Panel::Settings, "Settings");

        // Fill remaining space
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(10.0);
            ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
        });
    });
}

fn nav_button(ui: &mut egui::Ui, app: &mut App, panel: Panel, label: &str) {
    let is_selected = app.current_panel == panel;

    let button = egui::Button::new(label)
        .min_size(egui::vec2(160.0, 32.0));

    let response = if is_selected {
        ui.add(button.fill(ui.style().visuals.selection.bg_fill))
    } else {
        ui.add(button)
    };

    if response.clicked() {
        app.current_panel = panel;
    }
}
```

### 15.4 Status Bar

**`src/ui/status_bar.rs`**

```rust
use crate::app::App;
use eframe::egui;

pub fn show(app: &App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        // Database status
        ui.label(format!("DB: {} employees", app.employees.len()));

        ui.separator();

        // Departments count
        ui.label(format!("{} departments", app.departments.len()));

        ui.separator();

        // Sync status
        if app.is_syncing {
            ui.label(format!("Syncing: {:.0}%", app.sync_progress * 100.0));
        } else {
            ui.label(&app.sync_status);
        }

        // Loading indicator
        if app.is_loading {
            ui.separator();
            ui.spinner();
            ui.label(&app.loading_message);
        }
    });
}
```

### 15.5 UI Module Structure

**`src/ui/mod.rs`**

```rust
pub mod sidebar;
pub mod status_bar;
pub mod menu_bar;
pub mod dashboard;
pub mod departments;
pub mod employees;
pub mod sync_panel;
pub mod reports;
pub mod settings;
```

---

## Deliverables

- [ ] main.rs with eframe setup
- [ ] App implements eframe::App
- [ ] Message processing loop
- [ ] Modal dialogs (error, success, confirm)
- [ ] Sidebar navigation
- [ ] Status bar
- [ ] UI module structure
