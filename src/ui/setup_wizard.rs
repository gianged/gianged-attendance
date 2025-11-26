//! First-run setup wizard for configuration.

use crate::config::AppConfig;
use eframe::egui::{self, Color32, RichText};
use std::sync::mpsc;

/// Connection test state.
#[derive(Default, Clone)]
pub enum ConnectionTestState {
    #[default]
    NotTested,
    Testing,
    Success,
    Failed(String),
}

/// Setup wizard state.
pub struct SetupWizard {
    /// Current step (0-4).
    pub current_step: usize,
    /// Configuration being built.
    pub config: AppConfig,
    /// Database connection test state.
    pub db_test_state: ConnectionTestState,
    /// Device connection test state.
    pub device_test_state: ConnectionTestState,
    /// Wizard completed flag.
    pub completed: bool,
    /// Port input as string for text editing.
    port_input: String,
    /// Days input as string.
    days_input: String,
    /// Max user ID input as string.
    max_user_id_input: String,
    /// Interval input as string.
    interval_input: String,
}

impl Default for SetupWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl SetupWizard {
    pub fn new() -> Self {
        let config = AppConfig::default();
        Self {
            current_step: 0,
            port_input: config.database.port.to_string(),
            days_input: config.sync.days.to_string(),
            max_user_id_input: config.sync.max_user_id.to_string(),
            interval_input: config.sync.interval_minutes.to_string(),
            config,
            db_test_state: ConnectionTestState::NotTested,
            device_test_state: ConnectionTestState::NotTested,
            completed: false,
        }
    }

    /// Check if user can proceed to next step.
    pub fn can_proceed(&self) -> bool {
        match self.current_step {
            0 => true, // Welcome - always can proceed
            1 => matches!(self.db_test_state, ConnectionTestState::Success),
            2 => true, // Device is optional
            3 => self.validate_sync_step().is_ok(),
            4 => true, // Confirmation
            _ => false,
        }
    }

    /// Validate sync step inputs.
    fn validate_sync_step(&self) -> Result<(), String> {
        if self.config.sync.days < 1 || self.config.sync.days > 365 {
            return Err("Days must be between 1 and 365".to_string());
        }
        if self.config.sync.max_user_id < 1 {
            return Err("Max user ID must be at least 1".to_string());
        }
        if self.config.sync.interval_minutes < 1 {
            return Err("Interval must be at least 1 minute".to_string());
        }
        Ok(())
    }

    /// Get step title.
    fn step_title(&self) -> &'static str {
        match self.current_step {
            0 => "Welcome",
            1 => "Database Configuration",
            2 => "Device Configuration",
            3 => "Sync Settings",
            4 => "Confirmation",
            _ => "Setup",
        }
    }

    /// Total number of steps.
    const TOTAL_STEPS: usize = 5;
}

/// Setup wizard application.
pub struct SetupApp {
    pub wizard: SetupWizard,
    pub initial_error: Option<String>,
    pub rt: tokio::runtime::Runtime,
    db_test_rx: Option<mpsc::Receiver<Result<(), String>>>,
    device_test_rx: Option<mpsc::Receiver<Result<(), String>>>,
}

impl SetupApp {
    pub fn new(wizard: SetupWizard, initial_error: Option<String>) -> Self {
        Self {
            wizard,
            initial_error,
            rt: tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"),
            db_test_rx: None,
            device_test_rx: None,
        }
    }

    /// Test database connection asynchronously.
    fn start_db_test(&mut self) {
        let conn_str = self.wizard.config.database.connection_string();
        let (tx, rx) = mpsc::channel();
        self.db_test_rx = Some(rx);
        self.wizard.db_test_state = ConnectionTestState::Testing;

        self.rt.spawn(async move {
            let result = test_db_connection(&conn_str).await;
            let _ = tx.send(result);
        });
    }

    /// Test device connection asynchronously.
    #[allow(dead_code)]
    fn start_device_test(&mut self) {
        let url = self.wizard.config.device.url.clone();
        let (tx, rx) = mpsc::channel();
        self.device_test_rx = Some(rx);
        self.wizard.device_test_state = ConnectionTestState::Testing;

        self.rt.spawn(async move {
            let result = test_device_connection(&url).await;
            let _ = tx.send(result);
        });
    }

    /// Check for async test results.
    fn poll_test_results(&mut self) {
        if let Some(rx) = &self.db_test_rx
            && let Ok(result) = rx.try_recv()
        {
            self.wizard.db_test_state = match result {
                Ok(()) => ConnectionTestState::Success,
                Err(e) => ConnectionTestState::Failed(e),
            };
            self.db_test_rx = None;
        }

        if let Some(rx) = &self.device_test_rx
            && let Ok(result) = rx.try_recv()
        {
            self.wizard.device_test_state = match result {
                Ok(()) => ConnectionTestState::Success,
                Err(e) => ConnectionTestState::Failed(e),
            };
            self.device_test_rx = None;
        }
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll async test results
        self.poll_test_results();

        // Request repaint while testing
        if matches!(self.wizard.db_test_state, ConnectionTestState::Testing)
            || matches!(self.wizard.device_test_state, ConnectionTestState::Testing)
        {
            ctx.request_repaint();
        }

        // Show initial error dialog
        if let Some(err) = self.initial_error.clone() {
            egui::Window::new("Configuration Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.colored_label(Color32::from_rgb(255, 100, 100), &err);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.initial_error = None;
                    }
                });
            return;
        }

        // Main wizard panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                // Header
                ui.horizontal(|ui| {
                    ui.heading(RichText::new("Gianged Attendance Setup").size(24.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!(
                            "Step {} of {}",
                            self.wizard.current_step + 1,
                            SetupWizard::TOTAL_STEPS
                        ));
                    });
                });

                ui.separator();
                ui.add_space(10.0);

                // Step title
                ui.heading(self.wizard.step_title());
                ui.add_space(20.0);

                // Step content
                let needs_db_test = match self.wizard.current_step {
                    0 => {
                        show_welcome_step(ui);
                        false
                    }
                    1 => show_database_step(ui, &mut self.wizard),
                    2 => {
                        show_device_step(ui, &mut self.wizard);
                        false
                    }
                    3 => {
                        show_sync_step(ui, &mut self.wizard);
                        false
                    }
                    4 => {
                        show_confirmation_step(ui, &self.wizard);
                        false
                    }
                    _ => false,
                };

                if needs_db_test {
                    self.start_db_test();
                }

                ui.add_space(30.0);
                ui.separator();

                // Navigation buttons
                ui.horizontal(|ui| {
                    if self.wizard.current_step > 0 && ui.button("< Back").clicked() {
                        self.wizard.current_step -= 1;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.wizard.current_step < SetupWizard::TOTAL_STEPS - 1 {
                            let btn_text = if self.wizard.current_step == 0 {
                                "Get Started >"
                            } else {
                                "Next >"
                            };
                            let enabled = self.wizard.can_proceed();
                            if ui.add_enabled(enabled, egui::Button::new(btn_text)).clicked() {
                                self.wizard.current_step += 1;
                            }
                        } else {
                            // Final step - Save & Exit
                            if ui.button("Save & Exit").clicked() {
                                self.wizard.completed = true;
                            }
                        }
                    });
                });
            });
        });

        // Handle completion
        if self.wizard.completed {
            let path = AppConfig::default_path();
            match self.wizard.config.save(&path) {
                Ok(()) => {
                    // Show success and close
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(e) => {
                    self.initial_error = Some(format!("Failed to save config: {}", e));
                    self.wizard.completed = false;
                }
            }
        }
    }
}

fn show_welcome_step(ui: &mut egui::Ui) {
    ui.label("Welcome to Gianged Attendance!");
    ui.add_space(10.0);
    ui.label("This wizard will help you configure the application.");
    ui.add_space(20.0);
    ui.label("You will need:");
    ui.add_space(5.0);
    ui.label("  - PostgreSQL database connection details");
    ui.label("  - ZKTeco device IP address (optional)");
}

fn show_database_step(ui: &mut egui::Ui, wizard: &mut SetupWizard) -> bool {
    let mut needs_test = false;

    egui::Grid::new("db_grid")
        .num_columns(2)
        .spacing([20.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut wizard.config.database.host);
            ui.end_row();

            ui.label("Port:");
            if ui.text_edit_singleline(&mut wizard.port_input).changed()
                && let Ok(p) = wizard.port_input.parse()
            {
                wizard.config.database.port = p;
            }
            ui.end_row();

            ui.label("Database:");
            ui.text_edit_singleline(&mut wizard.config.database.name);
            ui.end_row();

            ui.label("Username:");
            ui.text_edit_singleline(&mut wizard.config.database.username);
            ui.end_row();

            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut wizard.config.database.password).password(true));
            ui.end_row();
        });

    ui.add_space(20.0);

    ui.horizontal(|ui| {
        let testing = matches!(wizard.db_test_state, ConnectionTestState::Testing);
        if ui.add_enabled(!testing, egui::Button::new("Test Connection")).clicked() {
            needs_test = true;
        }

        ui.add_space(10.0);

        match &wizard.db_test_state {
            ConnectionTestState::NotTested => {
                ui.label("Not tested");
            }
            ConnectionTestState::Testing => {
                ui.spinner();
                ui.label("Testing...");
            }
            ConnectionTestState::Success => {
                ui.colored_label(Color32::from_rgb(100, 200, 100), "Connection successful!");
            }
            ConnectionTestState::Failed(e) => {
                ui.colored_label(Color32::from_rgb(255, 100, 100), format!("Failed: {}", e));
            }
        }
    });

    needs_test
}

fn show_device_step(ui: &mut egui::Ui, wizard: &mut SetupWizard) {
    ui.label("Configure the ZKTeco fingerprint scanner connection.");
    ui.label(RichText::new("This step is optional - you can configure it later.").italics());
    ui.add_space(10.0);

    egui::Grid::new("device_grid")
        .num_columns(2)
        .spacing([20.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Device URL:");
            ui.text_edit_singleline(&mut wizard.config.device.url);
            ui.end_row();

            ui.label("Username:");
            ui.text_edit_singleline(&mut wizard.config.device.username);
            ui.end_row();

            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut wizard.config.device.password).password(true));
            ui.end_row();
        });

    ui.add_space(10.0);

    match &wizard.device_test_state {
        ConnectionTestState::Success => {
            ui.colored_label(Color32::from_rgb(100, 200, 100), "Device reachable!");
        }
        ConnectionTestState::Failed(e) => {
            ui.colored_label(
                Color32::from_rgb(255, 200, 100),
                format!("Device not reachable: {} (you can still continue)", e),
            );
        }
        _ => {}
    }
}

fn show_sync_step(ui: &mut egui::Ui, wizard: &mut SetupWizard) {
    ui.label("Configure attendance sync settings.");
    ui.add_space(10.0);

    egui::Grid::new("sync_grid")
        .num_columns(2)
        .spacing([20.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Days to sync:");
            if ui.text_edit_singleline(&mut wizard.days_input).changed()
                && let Ok(d) = wizard.days_input.parse()
            {
                wizard.config.sync.days = d;
            }
            ui.end_row();

            ui.label("Max user ID:");
            if ui.text_edit_singleline(&mut wizard.max_user_id_input).changed()
                && let Ok(m) = wizard.max_user_id_input.parse()
            {
                wizard.config.sync.max_user_id = m;
            }
            ui.end_row();

            ui.label("Auto-sync:");
            ui.checkbox(&mut wizard.config.sync.auto_enabled, "Enable automatic sync");
            ui.end_row();

            ui.label("Interval (minutes):");
            if ui.text_edit_singleline(&mut wizard.interval_input).changed()
                && let Ok(i) = wizard.interval_input.parse()
            {
                wizard.config.sync.interval_minutes = i;
            }
            ui.end_row();
        });

    // Validation feedback
    if let Err(e) = wizard.validate_sync_step() {
        ui.add_space(10.0);
        ui.colored_label(Color32::from_rgb(255, 100, 100), e);
    }
}

fn show_confirmation_step(ui: &mut egui::Ui, wizard: &SetupWizard) {
    ui.label("Review your configuration:");
    ui.add_space(10.0);

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Database");
        ui.label(format!(
            "  {}@{}:{}/{}",
            wizard.config.database.username,
            wizard.config.database.host,
            wizard.config.database.port,
            wizard.config.database.name
        ));
    });

    ui.add_space(10.0);

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Device");
        if wizard.config.device.url.is_empty() {
            ui.label("  Not configured");
        } else {
            ui.label(format!("  {}", wizard.config.device.url));
        }
    });

    ui.add_space(10.0);

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Sync Settings");
        ui.label(format!("  Days: {}", wizard.config.sync.days));
        ui.label(format!("  Max user ID: {}", wizard.config.sync.max_user_id));
        ui.label(format!(
            "  Auto-sync: {}",
            if wizard.config.sync.auto_enabled {
                "Enabled"
            } else {
                "Disabled"
            }
        ));
        if wizard.config.sync.auto_enabled {
            ui.label(format!("  Interval: {} minutes", wizard.config.sync.interval_minutes));
        }
    });

    ui.add_space(20.0);
    ui.label("Click 'Save & Exit' to save and close the wizard.");
    ui.label("You will need to restart the application after setup.");
}

/// Test database connection.
async fn test_db_connection(conn_str: &str) -> Result<(), String> {
    use sea_orm::Database;

    let conn = Database::connect(conn_str).await.map_err(|e| e.to_string())?;

    conn.ping().await.map_err(|e| e.to_string())
}

/// Test device connection (simple HTTP check).
#[allow(dead_code)]
async fn test_device_connection(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL is empty".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    client.get(url).send().await.map_err(|e| e.to_string())?;

    Ok(())
}
