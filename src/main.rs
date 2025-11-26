//! Gianged Attendance - Desktop mini ERP for staff and attendance management.

use std::path::PathBuf;

use clap::Parser;
use eframe::egui;
use gianged_attendance as app;

use app::config::{AppConfig, ConfigLoadResult};
use app::db;
use app::ui::{SetupApp, SetupWizard};

/// Desktop mini ERP for staff and attendance management.
#[derive(Parser)]
#[command(name = "gianged-attendance")]
struct Cli {
    /// Use config.toml from current directory (dev mode)
    #[arg(long)]
    dev: bool,
}

/// Application launch mode.
enum LaunchMode {
    /// Normal operation with valid config.
    Normal(AppConfig),
    /// Setup wizard for first run or invalid config.
    Setup(SetupWizard, Option<String>),
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Gianged Attendance starting...");

    // Determine config path based on mode
    let config_path = if cli.dev {
        tracing::info!("Dev mode: loading config from current directory");
        PathBuf::from("config.toml")
    } else {
        AppConfig::default_path()
    };
    tracing::info!("Config path: {:?}", config_path);

    let launch_mode = match AppConfig::try_load(&config_path) {
        ConfigLoadResult::Loaded(config) => {
            tracing::info!("Config loaded successfully");
            LaunchMode::Normal(config)
        }
        ConfigLoadResult::Missing => {
            tracing::info!("Config missing, starting setup wizard");
            LaunchMode::Setup(SetupWizard::new(), None)
        }
        ConfigLoadResult::Invalid(e) => {
            tracing::warn!("Config invalid: {}", e);
            LaunchMode::Setup(SetupWizard::new(), Some(e.to_string()))
        }
    };

    match launch_mode {
        LaunchMode::Normal(config) => run_main_app(config),
        LaunchMode::Setup(wizard, error) => run_setup_wizard(wizard, error),
    }
}

/// Run the setup wizard.
fn run_setup_wizard(wizard: SetupWizard, initial_error: Option<String>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Gianged Attendance - Setup")
            .with_inner_size([600.0, 500.0])
            .with_min_inner_size([500.0, 400.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Gianged Attendance - Setup",
        options,
        Box::new(|_cc| Ok(Box::new(SetupApp::new(wizard, initial_error)))),
    )
}

/// Run the main application.
fn run_main_app(config: AppConfig) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Gianged Attendance")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    // Connect to database
    let pool = rt.block_on(async {
        let conn = db::connect(&config.database.connection_string())
            .await
            .expect("Failed to connect to database");

        // Log connection info
        if let Ok(version) = db::get_version(&conn).await {
            tracing::info!("PostgreSQL: {}", version);
        }

        if let Ok(counts) = db::get_table_counts(&conn).await {
            tracing::info!(
                "Tables: {} departments, {} employees, {} attendance logs",
                counts.departments,
                counts.employees,
                counts.attendance_logs
            );
        }

        conn
    });

    eframe::run_native(
        "Gianged Attendance",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MainApp::new(pool, config, rt)))
        }),
    )
}

/// Main application state.
struct MainApp {
    #[allow(dead_code)]
    pool: sea_orm::DatabaseConnection,
    #[allow(dead_code)]
    config: AppConfig,
    #[allow(dead_code)]
    rt: tokio::runtime::Runtime,
}

impl MainApp {
    fn new(pool: sea_orm::DatabaseConnection, config: AppConfig, rt: tokio::runtime::Runtime) -> Self {
        Self { pool, config, rt }
    }
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.heading("Gianged Attendance");
                ui.add_space(20.0);
                ui.label("Main application - placeholder");
                ui.add_space(10.0);
                ui.label(format!("Database: {}", self.config.database.name));
                ui.label(format!(
                    "Device: {}",
                    if self.config.device.url.is_empty() {
                        "Not configured"
                    } else {
                        &self.config.device.url
                    }
                ));
            });
        });
    }
}
