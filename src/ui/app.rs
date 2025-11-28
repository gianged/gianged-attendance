//! Main application UI.

use chrono::{DateTime, Local, NaiveDate};
use eframe::egui::{self, Align, Layout, ProgressBar};
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::db;
use crate::entities::{departments, employees};
use crate::models::attendance::DailyAttendance;
use crate::models::department::{CreateDepartment, UpdateDepartment};
use crate::models::employee::{CreateEmployee, UpdateEmployee};
use crate::sync::{SyncResult, run_sync_background};

use super::components::colors;
use super::{dashboard, department_panel, staff_panel, sync_panel};

/// Current panel being displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Panel {
    #[default]
    Dashboard,
    Departments,
    Employees,
    Sync,
    Reports,
    Settings,
}

impl Panel {
    /// Get the display name for the panel.
    pub fn name(&self) -> &'static str {
        match self {
            Panel::Dashboard => "Dashboard",
            Panel::Departments => "Departments",
            Panel::Employees => "Employees",
            Panel::Sync => "Sync",
            Panel::Reports => "Reports",
            Panel::Settings => "Settings",
        }
    }
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

/// Messages from async tasks to UI.
pub enum UiMessage {
    // Data loading
    DepartmentsLoaded(Vec<departments::Model>),
    EmployeesLoaded(Vec<employees::Model>),
    AttendanceLoaded(Vec<DailyAttendance>),
    LoadError(String),

    // Sync
    SyncProgress(f32, String),
    SyncCompleted(SyncResult),
    SyncFailed(String),

    // CRUD operations
    DepartmentSaved(departments::Model),
    DepartmentDeleted(i32),
    EmployeeSaved(employees::Model),
    EmployeeDeleted(i32),
    OperationFailed(String),

    // Export
    ExportCompleted(String),
    ExportFailed(String),

    // Connection tests
    DeviceTestResult(bool),
    DatabaseTestResult(bool),
}

/// Form state for department CRUD.
#[derive(Default, Clone)]
pub struct DepartmentForm {
    pub id: Option<i32>,
    pub name: String,
    pub parent_id: Option<i32>,
    pub display_order: String,
    pub is_active: bool,
    pub is_open: bool,
    pub is_editing: bool,
}

impl DepartmentForm {
    /// Reset the form to default values.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Create a form pre-filled for editing an existing department.
    pub fn edit(dept: &departments::Model) -> Self {
        Self {
            id: Some(dept.id),
            name: dept.name.clone(),
            parent_id: dept.parent_id,
            display_order: dept.display_order.to_string(),
            is_active: dept.is_active,
            is_open: true,
            is_editing: true,
        }
    }
}

/// Form state for employee CRUD.
#[derive(Default, Clone)]
pub struct EmployeeForm {
    pub id: Option<i32>,
    pub employee_code: String,
    pub full_name: String,
    pub department_id: Option<i32>,
    pub device_uid: String,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub is_open: bool,
    pub is_editing: bool,
}

impl EmployeeForm {
    /// Reset the form to default values.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Create a form pre-filled for editing an existing employee.
    pub fn edit(emp: &employees::Model) -> Self {
        Self {
            id: Some(emp.id),
            employee_code: emp.employee_code.clone(),
            full_name: emp.full_name.clone(),
            department_id: emp.department_id,
            device_uid: emp.device_uid.map(|u| u.to_string()).unwrap_or_default(),
            gender: emp.gender.clone(),
            birth_date: emp.birth_date,
            start_date: Some(emp.start_date),
            end_date: None,
            is_active: emp.is_active,
            is_open: true,
            is_editing: true,
        }
    }
}

/// Filter state for reports.
#[derive(Clone)]
pub struct ReportFilter {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub department_id: Option<i32>,
    pub employee_id: Option<i32>,
}

impl Default for ReportFilter {
    fn default() -> Self {
        let today = Local::now().date_naive();
        Self {
            start_date: today - chrono::Duration::days(30),
            end_date: today,
            department_id: None,
            employee_id: None,
        }
    }
}

/// Log level for UI messages.
#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Log entry for display in the UI.
#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub message: String,
    pub level: LogLevel,
}

/// Target for delete confirmation dialog.
#[derive(Clone)]
pub enum DeleteTarget {
    Department(i32, String),
    Employee(i32, String),
}

/// Main application state.
pub struct App {
    // Runtime and database
    pub rt: tokio::runtime::Runtime,
    pub pool: DatabaseConnection,

    // Message channel for async communication
    pub tx: mpsc::UnboundedSender<UiMessage>,
    pub rx: mpsc::UnboundedReceiver<UiMessage>,

    // Navigation
    pub current_panel: Panel,

    // Cached data
    pub departments: Vec<departments::Model>,
    pub employees: Vec<employees::Model>,
    pub attendance: Vec<DailyAttendance>,

    // Loading states
    pub is_loading: bool,
    pub loading_message: String,

    // Forms
    pub department_form: DepartmentForm,
    pub employee_form: EmployeeForm,
    pub report_filter: ReportFilter,

    // Sync state
    pub sync_progress: f32,
    pub sync_status: String,
    pub is_syncing: bool,
    pub last_sync_time: Option<DateTime<Local>>,

    // Sync state (used by dashboard and sync panel)
    pub sync_state: SyncState,
    sync_progress_rx: Option<mpsc::UnboundedReceiver<SyncProgress>>,

    // Log messages
    pub log_messages: Vec<LogEntry>,

    // Configuration
    pub config: AppConfig,
    pub config_modified: bool,

    // Search/filter state
    pub employee_search: String,
    pub employee_dept_filter: Option<i32>,

    // Dialogs
    pub show_delete_confirm: bool,
    pub delete_target: Option<DeleteTarget>,
    pub error_message: Option<String>,
    pub success_message: Option<String>,

    // Scanner dialog
    pub scanner_dialog_open: bool,
    pub scanner_url_input: String,
    scanner_test_rx: Option<mpsc::UnboundedReceiver<Result<(), String>>>,
    scanner_test_status: Option<Result<(), String>>,

    // Device state
    pub device_status: DeviceStatus,
    device_status_rx: Option<mpsc::UnboundedReceiver<Result<(), String>>>,
}

impl App {
    pub fn new(pool: DatabaseConnection, config: AppConfig, rt: tokio::runtime::Runtime) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let scanner_url_input = config.device.url.clone();

        let mut app = Self {
            rt,
            pool,
            tx,
            rx,
            current_panel: Panel::default(),
            departments: Vec::new(),
            employees: Vec::new(),
            attendance: Vec::new(),
            is_loading: false,
            loading_message: String::new(),
            department_form: DepartmentForm::default(),
            employee_form: EmployeeForm::default(),
            report_filter: ReportFilter::default(),
            sync_progress: 0.0,
            sync_status: "Ready".to_string(),
            is_syncing: false,
            last_sync_time: None,
            sync_state: SyncState::default(),
            sync_progress_rx: None,
            log_messages: Vec::new(),
            config,
            config_modified: false,
            employee_search: String::new(),
            employee_dept_filter: None,
            show_delete_confirm: false,
            delete_target: None,
            error_message: None,
            success_message: None,
            scanner_dialog_open: false,
            scanner_url_input,
            scanner_test_rx: None,
            scanner_test_status: None,
            device_status: DeviceStatus::Disconnected,
            device_status_rx: None,
        };

        // Load initial data
        app.load_departments();
        app.load_employees();

        app
    }

    /// Log a message to the UI log.
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.log_messages.push(LogEntry {
            timestamp: Local::now(),
            message: message.into(),
            level,
        });

        // Keep only last 100 messages
        if self.log_messages.len() > 100 {
            self.log_messages.remove(0);
        }
    }

    /// Log an info message.
    pub fn log_info(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    /// Log a success message.
    pub fn log_success(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Success, message);
    }

    /// Log a warning message.
    pub fn log_warning(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Warning, message);
    }

    /// Log an error message.
    pub fn log_error(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }

    /// Load departments from database.
    pub fn load_departments(&mut self) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::department::list_all(&pool).await {
                Ok(depts) => {
                    let _ = tx.send(UiMessage::DepartmentsLoaded(depts));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    /// Load employees from database.
    pub fn load_employees(&mut self) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::employee::list_all(&pool).await {
                Ok(emps) => {
                    let _ = tx.send(UiMessage::EmployeesLoaded(emps));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    /// Load attendance data from database.
    pub fn load_attendance(&mut self) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();
        let start_date = self.report_filter.start_date;
        let end_date = self.report_filter.end_date;

        self.rt.spawn(async move {
            match db::attendance::get_daily_summary(&pool, start_date, end_date).await {
                Ok(attendance) => {
                    let _ = tx.send(UiMessage::AttendanceLoaded(attendance));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    /// Create a new department.
    pub fn create_department(&mut self, data: CreateDepartment) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::department::create(&pool, data).await {
                Ok(dept) => {
                    let _ = tx.send(UiMessage::DepartmentSaved(dept));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    /// Update an existing department.
    pub fn update_department(&mut self, id: i32, data: UpdateDepartment) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::department::update(&pool, id, data).await {
                Ok(Some(dept)) => {
                    let _ = tx.send(UiMessage::DepartmentSaved(dept));
                }
                Ok(None) => {
                    let _ = tx.send(UiMessage::OperationFailed("Department not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    /// Delete a department.
    pub fn delete_department(&mut self, id: i32) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::department::delete(&pool, id).await {
                Ok(true) => {
                    let _ = tx.send(UiMessage::DepartmentDeleted(id));
                }
                Ok(false) => {
                    let _ = tx.send(UiMessage::OperationFailed("Department not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    /// Create a new employee.
    pub fn create_employee(&mut self, data: CreateEmployee) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::employee::create(&pool, data).await {
                Ok(emp) => {
                    let _ = tx.send(UiMessage::EmployeeSaved(emp));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    /// Update an existing employee.
    pub fn update_employee(&mut self, id: i32, data: UpdateEmployee) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::employee::update(&pool, id, data).await {
                Ok(Some(emp)) => {
                    let _ = tx.send(UiMessage::EmployeeSaved(emp));
                }
                Ok(None) => {
                    let _ = tx.send(UiMessage::OperationFailed("Employee not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    /// Delete an employee.
    pub fn delete_employee(&mut self, id: i32) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::employee::delete(&pool, id).await {
                Ok(true) => {
                    let _ = tx.send(UiMessage::EmployeeDeleted(id));
                }
                Ok(false) => {
                    let _ = tx.send(UiMessage::OperationFailed("Employee not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    /// Export employees to Excel.
    pub fn export_employees(&mut self) {
        let filename = crate::export::generate_export_filename("employees");
        let path = std::path::PathBuf::from(&filename);

        match crate::export::export_employees_to_excel(&self.employees, &self.departments, &path) {
            Ok(()) => {
                self.success_message = Some(format!("Exported to: {}", filename));
                self.log_success(format!("Exported employees: {}", filename));
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
                self.log_error(format!("Export failed: {}", e));
            }
        }
    }

    /// Test device connection.
    pub fn test_device_connection(&mut self) {
        self.log_info("Testing device connection...");

        let url = self.config.device.url.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            let client = crate::client::ZkClient::new(&url);
            match client.test_connection().await {
                Ok(success) => {
                    let _ = tx.send(UiMessage::DeviceTestResult(success));
                }
                Err(_) => {
                    let _ = tx.send(UiMessage::DeviceTestResult(false));
                }
            }
        });
    }

    /// Clear the activity log.
    pub fn clear_log(&mut self) {
        self.log_messages.clear();
    }

    /// Start device connection test (legacy).
    fn connect_device(&mut self) {
        let url = self.config.device.url.clone();
        if url.is_empty() {
            self.device_status = DeviceStatus::Error;
            return;
        }

        let (tx, rx) = mpsc::unbounded_channel();
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

        let (tx, rx) = mpsc::unbounded_channel();
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
    pub fn start_sync(&mut self) {
        let (tx, rx) = mpsc::unbounded_channel();
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

    /// Export today's attendance report to Excel.
    pub fn export_today_report(&mut self) {
        let today = chrono::Local::now().date_naive();
        let data: Vec<_> = self
            .attendance
            .iter()
            .filter(|a| a.work_date == today)
            .cloned()
            .collect();

        if data.is_empty() {
            self.error_message = Some("No attendance data for today".to_string());
            return;
        }

        let filename = crate::export::generate_export_filename("attendance_today");
        let path = std::path::PathBuf::from(&filename);

        match crate::export::export_attendance_summary_to_excel(&data, &path) {
            Ok(()) => {
                self.success_message = Some(format!("Exported to: {}", filename));
                self.log_success(format!("Exported today's report: {}", filename));
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
                self.log_error(format!("Export failed: {}", e));
            }
        }
    }

    /// Poll async operation results.
    fn poll_async_results(&mut self) {
        // Poll UiMessage channel
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UiMessage::DepartmentsLoaded(deps) => {
                    self.departments = deps;
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
                UiMessage::LoadError(e) => {
                    self.error_message = Some(e.clone());
                    self.log_error(e);
                    self.is_loading = false;
                }
                UiMessage::SyncProgress(progress, message) => {
                    self.sync_progress = progress;
                    self.sync_status = message;
                }
                UiMessage::SyncCompleted(result) => {
                    self.is_syncing = false;
                    self.last_sync_time = Some(Local::now());
                    self.log_success(format!(
                        "Sync completed: {} inserted, {} skipped",
                        result.inserted, result.skipped
                    ));
                }
                UiMessage::SyncFailed(e) => {
                    self.is_syncing = false;
                    self.error_message = Some(e.clone());
                    self.log_error(e);
                }
                UiMessage::DepartmentSaved(dept) => {
                    self.success_message = Some(format!("Department '{}' saved", dept.name));
                    self.department_form.reset();
                    self.load_departments();
                }
                UiMessage::DepartmentDeleted(id) => {
                    self.departments.retain(|d| d.id != id);
                    self.success_message = Some("Department deleted".to_string());
                    self.log_success("Department deleted");
                }
                UiMessage::EmployeeSaved(emp) => {
                    self.success_message = Some(format!("Employee '{}' saved", emp.full_name));
                    self.employee_form.reset();
                    self.load_employees();
                }
                UiMessage::EmployeeDeleted(id) => {
                    self.employees.retain(|e| e.id != id);
                    self.success_message = Some("Employee deleted".to_string());
                    self.log_success("Employee deleted");
                }
                UiMessage::OperationFailed(e) => {
                    self.error_message = Some(e.clone());
                    self.log_error(e);
                }
                UiMessage::ExportCompleted(path) => {
                    self.success_message = Some(format!("Exported to {}", path));
                    self.log_success(format!("Export completed: {}", path));
                }
                UiMessage::ExportFailed(e) => {
                    self.error_message = Some(e.clone());
                    self.log_error(e);
                }
                UiMessage::DeviceTestResult(ok) => {
                    if ok {
                        self.device_status = DeviceStatus::Connected;
                        self.log_success("Device connection successful");
                    } else {
                        self.device_status = DeviceStatus::Error;
                        self.log_error("Device connection failed");
                    }
                }
                UiMessage::DatabaseTestResult(_ok) => {
                    // Handle if needed
                }
            }
        }

        // Poll device connection (legacy)
        if let Some(mut rx) = self.device_status_rx.take() {
            match rx.try_recv() {
                Ok(result) => {
                    self.device_status = match result {
                        Ok(()) => DeviceStatus::Connected,
                        Err(_) => DeviceStatus::Error,
                    };
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    self.device_status_rx = Some(rx);
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed, keep None
                }
            }
        }

        // Poll scanner test (legacy)
        if let Some(mut rx) = self.scanner_test_rx.take() {
            match rx.try_recv() {
                Ok(result) => {
                    self.scanner_test_status = Some(result);
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    self.scanner_test_rx = Some(rx);
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed, keep None
                }
            }
        }

        // Poll sync progress (legacy)
        if let Some(mut rx) = self.sync_progress_rx.take() {
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

                    if self.scanner_test_rx.is_some() {
                        ui.spinner();
                        ui.label("Testing...");
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

    /// Render modal dialogs (error, success, delete confirmation).
    fn show_dialogs(&mut self, ctx: &egui::Context) {
        // Error dialog
        if let Some(ref error) = self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.colored_label(colors::ERROR, error);
                    ui.add_space(10.0);
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
                    ui.colored_label(colors::SUCCESS, msg);
                    ui.add_space(10.0);
                    if ui.button("OK").clicked() {
                        self.success_message = None;
                    }
                });
        }

        // Delete confirmation dialog
        if self.show_delete_confirm
            && let Some(ref target) = self.delete_target.clone()
        {
            let (title, message) = match target {
                DeleteTarget::Department(_, name) => ("Delete Department", format!("Delete department '{}'?", name)),
                DeleteTarget::Employee(_, name) => ("Delete Employee", format!("Delete employee '{}'?", name)),
            };

            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.add_space(10.0);
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

    /// Execute the confirmed delete operation.
    fn confirm_delete(&mut self) {
        if let Some(target) = self.delete_target.take() {
            match target {
                DeleteTarget::Department(id, name) => {
                    self.log_info(format!("Deleting department: {}", name));
                    self.delete_department(id);
                }
                DeleteTarget::Employee(id, name) => {
                    self.log_info(format!("Deleting employee: {}", name));
                    self.delete_employee(id);
                }
            }
        }
    }
}

impl eframe::App for App {
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

        // Modal dialogs (error, success, delete confirmation)
        self.show_dialogs(ctx);

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| match self.current_panel {
            Panel::Dashboard => {
                if let Some(next) = dashboard::show(self, ui) {
                    self.current_panel = next;
                }
            }
            Panel::Departments => {
                if department_panel::show(self, ui) {
                    self.current_panel = Panel::Dashboard;
                }
            }
            Panel::Employees => {
                if staff_panel::show(self, ui) {
                    self.current_panel = Panel::Dashboard;
                }
            }
            Panel::Sync => {
                if sync_panel::show(self, ui) {
                    self.current_panel = Panel::Dashboard;
                }
            }
            Panel::Reports => {
                // TODO: Implement reports panel
                ui.centered_and_justified(|ui| {
                    ui.label("Reports - Coming Soon");
                });
            }
            Panel::Settings => {
                // TODO: Implement settings panel
                ui.centered_and_justified(|ui| {
                    ui.label("Settings - Coming Soon");
                });
            }
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
