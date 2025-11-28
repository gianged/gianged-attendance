//! Main application UI.

use std::sync::mpsc;

use chrono::{DateTime, Local};
use eframe::egui::{self, Align, Layout, ProgressBar};
use sea_orm::DatabaseConnection;

use crate::config::AppConfig;
use crate::sync::run_sync_background;

use super::components::colors;
use super::{dashboard, department_panel, staff_panel, sync_panel};

/// Current panel being displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurrentPanel {
    #[default]
    Dashboard,
    Departments,
    Staff,
    Sync,
}

/// Device connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Sync operation state.
#[derive(Debug, Clone, Default)]
pub enum SyncState {
    #[default]
    Idle,
    InProgress {
        progress: f32,
        message: String,
    },
    Completed {
        records_synced: u32,
    },
    Error(String),
}

/// Sync progress message from async task.
pub enum SyncProgress {
    Started,
    Progress { percent: f32, message: String },
    Completed { records: u32, timestamp: DateTime<Local> },
    Error(String),
}

/// Main application state.
pub struct MainApp {
    pool: DatabaseConnection,
    config: AppConfig,
    rt: tokio::runtime::Runtime,

    // UI navigation
    current_panel: CurrentPanel,

    // Device state
    device_status: DeviceStatus,
    device_status_rx: Option<mpsc::Receiver<Result<(), String>>>,

    // Sync state
    sync_state: SyncState,
    last_sync_time: Option<DateTime<Local>>,
    sync_progress_rx: Option<mpsc::Receiver<SyncProgress>>,

    // Scanner dialog
    scanner_dialog_open: bool,
    scanner_url_input: String,
    scanner_test_rx: Option<mpsc::Receiver<Result<(), String>>>,
    scanner_test_status: Option<Result<(), String>>,
}

impl MainApp {
    pub fn new(pool: DatabaseConnection, config: AppConfig, rt: tokio::runtime::Runtime) -> Self {
        let scanner_url_input = config.device.url.clone();
        Self {
            pool,
            config,
            rt,
            current_panel: CurrentPanel::default(),
            device_status: DeviceStatus::Disconnected,
            device_status_rx: None,
            sync_state: SyncState::default(),
            last_sync_time: None,
            sync_progress_rx: None,
            scanner_dialog_open: false,
            scanner_url_input,
            scanner_test_rx: None,
            scanner_test_status: None,
        }
    }

    /// Start device connection test.
    fn connect_device(&mut self) {
        let url = self.config.device.url.clone();
        if url.is_empty() {
            self.device_status = DeviceStatus::Error;
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.device_status_rx = Some(rx);
        self.device_status = DeviceStatus::Connecting;

        self.rt.spawn(async move {
            let result = test_device_connection(&url).await;
            let _ = tx.send(result);
        });
    }

    /// Disconnect device (just update status).
    fn disconnect_device(&mut self) {
        self.device_status = DeviceStatus::Disconnected;
        self.device_status_rx = None;
    }

    /// Start scanner configuration test.
    fn test_scanner_connection(&mut self) {
        let url = self.scanner_url_input.clone();
        if url.is_empty() {
            self.scanner_test_status = Some(Err("URL is empty".to_string()));
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.scanner_test_rx = Some(rx);
        self.scanner_test_status = None;

        self.rt.spawn(async move {
            let result = test_device_connection(&url).await;
            let _ = tx.send(result);
        });
    }

    /// Save scanner configuration.
    fn save_scanner_config(&mut self) {
        self.config.device.url = self.scanner_url_input.clone();

        // Save to config file
        let path = AppConfig::default_path();
        if let Err(e) = self.config.save(&path) {
            tracing::error!("Failed to save config: {}", e);
        }
    }

    /// Start sync operation.
    fn start_sync(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.sync_progress_rx = Some(rx);
        self.sync_state = SyncState::InProgress {
            progress: 0.0,
            message: "Starting...".to_string(),
        };

        let config = self.config.clone();
        let db = self.pool.clone();

        self.rt.spawn(async move {
            let _ = tx.send(SyncProgress::Started);
            run_sync_background(config, db, tx).await;
        });
    }

    /// Poll async operation results.
    fn poll_async_results(&mut self) {
        // Poll device connection
        if let Some(rx) = &self.device_status_rx
            && let Ok(result) = rx.try_recv()
        {
            self.device_status = match result {
                Ok(()) => DeviceStatus::Connected,
                Err(_) => DeviceStatus::Error,
            };
            self.device_status_rx = None;
        }

        // Poll scanner test
        if let Some(rx) = &self.scanner_test_rx
            && let Ok(result) = rx.try_recv()
        {
            self.scanner_test_status = Some(result);
            self.scanner_test_rx = None;
        }

        // Poll sync progress
        if let Some(rx) = self.sync_progress_rx.take() {
            let mut done = false;
            while let Ok(progress) = rx.try_recv() {
                match progress {
                    SyncProgress::Started => {
                        self.sync_state = SyncState::InProgress {
                            progress: 0.0,
                            message: "Connecting to device...".to_string(),
                        };
                    }
                    SyncProgress::Progress { percent, message } => {
                        self.sync_state = SyncState::InProgress {
                            progress: percent,
                            message,
                        };
                    }
                    SyncProgress::Completed { records, timestamp } => {
                        self.sync_state = SyncState::Completed {
                            records_synced: records,
                        };
                        self.last_sync_time = Some(timestamp);
                        done = true;
                    }
                    SyncProgress::Error(e) => {
                        self.sync_state = SyncState::Error(e);
                        done = true;
                    }
                }
            }
            if !done {
                self.sync_progress_rx = Some(rx);
            }
        }
    }

    /// Render menu bar.
    fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Tools", |ui| {
                    if ui.button("Manage Scanner").clicked() {
                        self.scanner_dialog_open = true;
                        self.scanner_url_input = self.config.device.url.clone();
                        self.scanner_test_status = None;
                        ui.close();
                    }
                    ui.separator();
                    let connect_enabled =
                        !matches!(self.device_status, DeviceStatus::Connecting | DeviceStatus::Connected);
                    if ui
                        .add_enabled(connect_enabled, egui::Button::new("Connect Device"))
                        .clicked()
                    {
                        self.connect_device();
                        ui.close();
                    }
                    let disconnect_enabled = matches!(self.device_status, DeviceStatus::Connected);
                    if ui
                        .add_enabled(disconnect_enabled, egui::Button::new("Disconnect Device"))
                        .clicked()
                    {
                        self.disconnect_device();
                        ui.close();
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("General").clicked() {
                        // Future expansion
                        ui.close();
                    }
                });
            });
        });
    }

    /// Render status bar (display only, no interaction).
    fn show_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(28.0)
            .show(ctx, |ui| {
                ui.disable();
                ui.horizontal(|ui| {
                    // Device status (left side)
                    let (color, text) = match self.device_status {
                        DeviceStatus::Disconnected => (colors::NEUTRAL, "Disconnected"),
                        DeviceStatus::Connecting => (colors::WARNING, "Connecting..."),
                        DeviceStatus::Connected => (colors::SUCCESS, "Connected"),
                        DeviceStatus::Error => (colors::ERROR, "Connection Error"),
                    };

                    if matches!(self.device_status, DeviceStatus::Connecting) {
                        ui.spinner();
                    }
                    ui.colored_label(color, format!("Device: {}", text));

                    // Progress bar (right side)
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let SyncState::InProgress { progress, message } = &self.sync_state {
                            ui.add(
                                ProgressBar::new(*progress)
                                    .desired_width(250.0)
                                    .text(message)
                                    .animate(true),
                            );
                        }
                    });
                });
            });
    }

    /// Render scanner configuration dialog.
    fn show_scanner_dialog(&mut self, ctx: &egui::Context) {
        if !self.scanner_dialog_open {
            return;
        }

        let mut open = true;
        egui::Window::new("Scanner Configuration")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add_space(10.0);

                egui::Grid::new("scanner_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Device URL:");
                        ui.text_edit_singleline(&mut self.scanner_url_input);
                        ui.end_row();
                    });

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    let testing = self.scanner_test_rx.is_some();
                    if ui.add_enabled(!testing, egui::Button::new("Test Connection")).clicked() {
                        self.test_scanner_connection();
                    }

                    ui.add_space(10.0);

                    if let Some(rx) = &self.scanner_test_rx {
                        if rx.try_recv().is_err() {
                            ui.spinner();
                            ui.label("Testing...");
                        }
                    } else if let Some(result) = &self.scanner_test_status {
                        match result {
                            Ok(()) => {
                                ui.colored_label(colors::SUCCESS, "Connection successful!");
                            }
                            Err(e) => {
                                ui.colored_label(colors::ERROR, format!("Failed: {}", e));
                            }
                        }
                    }
                });

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.scanner_dialog_open = false;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Save").clicked() {
                            self.save_scanner_config();
                            self.scanner_dialog_open = false;
                        }
                    });
                });
            });

        if !open {
            self.scanner_dialog_open = false;
        }
    }
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll async results
        self.poll_async_results();

        // Request repaint during async operations
        if matches!(self.device_status, DeviceStatus::Connecting)
            || matches!(self.sync_state, SyncState::InProgress { .. })
            || self.scanner_test_rx.is_some()
        {
            ctx.request_repaint();
        }

        // Menu bar
        self.show_menu_bar(ctx);

        // Status bar
        self.show_status_bar(ctx);

        // Scanner dialog
        self.show_scanner_dialog(ctx);

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| match self.current_panel {
            CurrentPanel::Dashboard => {
                if let Some(next) = dashboard::show(ui) {
                    self.current_panel = next;
                }
            }
            CurrentPanel::Departments => {
                if department_panel::show(ui) {
                    self.current_panel = CurrentPanel::Dashboard;
                }
            }
            CurrentPanel::Staff => {
                if staff_panel::show(ui) {
                    self.current_panel = CurrentPanel::Dashboard;
                }
            }
            CurrentPanel::Sync => match sync_panel::show(ui, &self.sync_state, &self.last_sync_time) {
                sync_panel::Action::None => {}
                sync_panel::Action::GoBack => {
                    self.current_panel = CurrentPanel::Dashboard;
                }
                sync_panel::Action::StartSync => {
                    self.start_sync();
                }
            },
        });
    }
}

/// Test device connection (simple HTTP check).
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
