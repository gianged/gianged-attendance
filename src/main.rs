//! Gianged Attendance - Desktop mini ERP for staff and attendance management.

use std::path::{Path, PathBuf};

use clap::Parser;
use eframe::egui;
use gianged_attendance as app;

use app::config::{AppConfig, ConfigLoadResult};
use app::db;
use app::ui::{App, SetupApp, SetupWizard};

/// Get the directory containing the executable.
fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Initialize logging based on build type.
/// - Debug: console output at INFO level
/// - Release: file output at WARN level
fn init_logging(exe_dir: &Path) {
    let log_dir = exe_dir.join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    #[cfg(debug_assertions)]
    {
        // Dev mode: console only
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
            )
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        // Release mode: file only, WARN level
        let file_appender = tracing_appender::rolling::daily(&log_dir, "app");
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env().add_directive(tracing::Level::WARN.into()),
            )
            .init();
    }
}

/// Remove log files older than the specified number of days.
fn cleanup_old_logs(log_dir: &Path, keep_days: i64) {
    let cutoff = chrono::Local::now() - chrono::Duration::days(keep_days);

    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Only delete files starting with "app." (our log prefix)
        if !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("app."))
        {
            continue;
        }

        let Ok(metadata) = path.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };

        let modified: chrono::DateTime<chrono::Local> = modified.into();
        if modified < cutoff {
            std::fs::remove_file(&path).ok();
        }
    }
}

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
    let exe_dir = get_exe_dir();

    // Initialize logging
    init_logging(&exe_dir);

    // Cleanup logs older than 10 days
    cleanup_old_logs(&exe_dir.join("logs"), 10);

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
            // Add Phosphor icons to fonts
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(pool, config, rt)))
        }),
    )
}
