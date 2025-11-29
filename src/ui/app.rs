//! Main application UI.

use chrono::{DateTime, Local, NaiveDate};
use eframe::egui::{self, Align, Layout, ProgressBar};
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::db;
use crate::entities::{departments, employees};
use crate::models::attendance::{AttendanceDetail, DailyAttendance};
use crate::models::department::{CreateDepartment, UpdateDepartment};
use crate::models::employee::{CreateEmployee, UpdateEmployee};
use crate::sync::{SyncResult, run_sync_background};

use super::components::colors;
use super::{dashboard, department_panel, reports_panel, settings_panel, staff_panel, sync_panel};

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
    AttendanceDetailsLoaded(Vec<AttendanceDetail>),
    // Pagination counts
    AttendanceCountLoaded(u64),
    AttendanceDetailsCountLoaded(u64),
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
    pub scanner_uid: String,
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
            scanner_uid: emp.scanner_uid.map(|u| u.to_string()).unwrap_or_default(),
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

/// Report type: Summary (daily totals) or Detail (every check).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportType {
    #[default]
    Summary,
    Detail,
}

/// Page size for paginated report queries.
pub const REPORT_PAGE_SIZE: u64 = 500;

/// Filter state for reports.
#[derive(Clone)]
pub struct ReportFilter {
    pub report_type: ReportType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub department_id: Option<i32>,
    pub employee_id: Option<i32>,
    // Pagination state
    pub current_page: u64,
    pub total_records: u64,
}

impl Default for ReportFilter {
    fn default() -> Self {
        let today = Local::now().date_naive();
        Self {
            report_type: ReportType::default(),
            start_date: today - chrono::Duration::days(30),
            end_date: today,
            department_id: None,
            employee_id: None,
            current_page: 0,
            total_records: 0,
        }
    }
}

impl ReportFilter {
    /// Calculate total pages based on current total records.
    pub fn total_pages(&self) -> u64 {
        if self.total_records == 0 {
            1
        } else {
            self.total_records.div_ceil(REPORT_PAGE_SIZE)
        }
    }

    /// Reset pagination when filters change.
    pub fn reset_pagination(&mut self) {
        self.current_page = 0;
        self.total_records = 0;
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
    pub attendance_details: Vec<AttendanceDetail>,

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

    // Settings panel test status
    pub device_test_status: Option<bool>,
    pub database_test_status: Option<bool>,
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
            attendance_details: Vec::new(),
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
            device_test_status: None,
            database_test_status: None,
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

    /// Generate report based on current filter settings.
    /// Uses paginated queries for better performance.
    pub fn generate_report(&mut self) {
        self.is_loading = true;
        self.loading_message = "Generating report...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();
        let filter = self.report_filter.clone();
        let pagination = db::attendance::Pagination::new(filter.current_page, REPORT_PAGE_SIZE);

        // Load counts first, then data
        let pool_count = pool.clone();
        let tx_count = tx.clone();
        let filter_count = filter.clone();

        // Get summary count
        self.rt.spawn(async move {
            match db::attendance::count_daily_summary(
                &pool_count,
                filter_count.start_date,
                filter_count.end_date,
                filter_count.department_id,
            )
            .await
            {
                Ok(count) => {
                    let _ = tx_count.send(UiMessage::AttendanceCountLoaded(count));
                }
                Err(e) => {
                    let _ = tx_count.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });

        // Get details count
        let pool_detail_count = pool.clone();
        let tx_detail_count = tx.clone();
        let filter_detail_count = filter.clone();

        self.rt.spawn(async move {
            match db::attendance::count_attendance_details(
                &pool_detail_count,
                filter_detail_count.start_date,
                filter_detail_count.end_date,
                filter_detail_count.department_id,
            )
            .await
            {
                Ok(count) => {
                    let _ = tx_detail_count.send(UiMessage::AttendanceDetailsCountLoaded(count));
                }
                Err(e) => {
                    let _ = tx_detail_count.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });

        // Load paginated summary data
        let pool_summary = pool.clone();
        let tx_summary = tx.clone();
        let filter_summary = filter.clone();
        let pagination_summary = pagination;

        self.rt.spawn(async move {
            match db::attendance::get_daily_summary_paginated(
                &pool_summary,
                filter_summary.start_date,
                filter_summary.end_date,
                filter_summary.department_id,
                pagination_summary,
            )
            .await
            {
                Ok(attendance) => {
                    let _ = tx_summary.send(UiMessage::AttendanceLoaded(attendance));
                }
                Err(e) => {
                    let _ = tx_summary.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });

        // Load paginated detail data
        self.rt.spawn(async move {
            match db::attendance::get_attendance_details_paginated(
                &pool,
                filter.start_date,
                filter.end_date,
                filter.department_id,
                pagination,
            )
            .await
            {
                Ok(details) => {
                    let _ = tx.send(UiMessage::AttendanceDetailsLoaded(details));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    /// Navigate to next page of report results.
    pub fn next_page(&mut self) {
        let total_pages = self.report_filter.total_pages();
        if self.report_filter.current_page + 1 < total_pages {
            self.report_filter.current_page += 1;
            self.generate_report();
        }
    }

    /// Navigate to previous page of report results.
    pub fn prev_page(&mut self) {
        if self.report_filter.current_page > 0 {
            self.report_filter.current_page -= 1;
            self.generate_report();
        }
    }

    /// Go to first page of report results.
    pub fn first_page(&mut self) {
        if self.report_filter.current_page != 0 {
            self.report_filter.current_page = 0;
            self.generate_report();
        }
    }

    /// Go to last page of report results.
    pub fn last_page(&mut self) {
        let total_pages = self.report_filter.total_pages();
        let last_page = if total_pages > 0 { total_pages - 1 } else { 0 };
        if self.report_filter.current_page != last_page {
            self.report_filter.current_page = last_page;
            self.generate_report();
        }
    }

    /// Export summary report to Excel.
    /// Fetches all data for the date range (not just paginated view).
    pub fn export_summary_report(&mut self) {
        self.is_loading = true;
        self.loading_message = "Exporting summary report...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();
        let filter = self.report_filter.clone();
        let filename = crate::export::generate_export_filename("attendance_summary");

        self.rt.spawn(async move {
            // Fetch all data for export (not paginated)
            let result = db::attendance::get_all_daily_summary_for_export(
                &pool,
                filter.start_date,
                filter.end_date,
                filter.department_id,
            )
            .await;

            match result {
                Ok(data) => {
                    if data.is_empty() {
                        let _ = tx.send(UiMessage::ExportFailed(
                            "No data to export. Generate a report first.".to_string(),
                        ));
                        return;
                    }

                    let path = std::path::PathBuf::from(&filename);
                    match crate::export::export_attendance_summary_to_excel(&data, &path) {
                        Ok(()) => {
                            let _ = tx.send(UiMessage::ExportCompleted(filename));
                        }
                        Err(e) => {
                            let _ = tx.send(UiMessage::ExportFailed(e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::ExportFailed(e.to_string()));
                }
            }
        });
    }

    /// Export detail report to Excel.
    /// Fetches all data for the date range (not just paginated view).
    pub fn export_detail_report(&mut self) {
        self.is_loading = true;
        self.loading_message = "Exporting detail report...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();
        let filter = self.report_filter.clone();
        let filename = crate::export::generate_export_filename("attendance_detail");

        self.rt.spawn(async move {
            // Fetch all data for export (not paginated)
            let result = db::attendance::get_all_attendance_details_for_export(
                &pool,
                filter.start_date,
                filter.end_date,
                filter.department_id,
            )
            .await;

            match result {
                Ok(data) => {
                    if data.is_empty() {
                        let _ = tx.send(UiMessage::ExportFailed(
                            "No data to export. Generate a report first.".to_string(),
                        ));
                        return;
                    }

                    let path = std::path::PathBuf::from(&filename);
                    match crate::export::export_attendance_detail_to_excel(&data, &path) {
                        Ok(()) => {
                            let _ = tx.send(UiMessage::ExportCompleted(filename));
                        }
                        Err(e) => {
                            let _ = tx.send(UiMessage::ExportFailed(e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::ExportFailed(e.to_string()));
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
                self.success_message = Some(format!("Exported to: {filename}"));
                self.log_success(format!("Exported employees: {filename}"));
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {e}"));
                self.log_error(format!("Export failed: {e}"));
            }
        }
    }

    /// Test device connection.
    pub fn test_device_connection(&mut self) {
        self.device_test_status = None;
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

    /// Test database connection.
    pub fn test_database_connection(&mut self) {
        self.database_test_status = None;
        self.log_info("Testing database connection...");

        let conn_str = self.config.database.connection_string();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match db::connect(&conn_str).await {
                Ok(pool) => match db::test_connection(&pool).await {
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

    /// Save configuration to file.
    pub fn save_config(&mut self) {
        let config_path = AppConfig::default_path();

        match self.config.save(&config_path) {
            Ok(()) => {
                self.config_modified = false;
                self.success_message = Some("Settings saved successfully".to_string());
                self.log_success("Settings saved");
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to save settings: {e}"));
                self.log_error(format!("Failed to save settings: {e}"));
            }
        }
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
                self.success_message = Some(format!("Exported to: {filename}"));
                self.log_success(format!("Exported today's report: {filename}"));
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {e}"));
                self.log_error(format!("Export failed: {e}"));
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
                UiMessage::AttendanceDetailsLoaded(details) => {
                    self.attendance_details = details;
                    self.is_loading = false;
                }
                UiMessage::AttendanceCountLoaded(count) => {
                    self.report_filter.total_records = count;
                }
                UiMessage::AttendanceDetailsCountLoaded(_count) => {
                    // Details count tracked separately if needed in future
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
                    self.success_message = Some(format!("Department '{name}' saved", name = dept.name));
                    self.department_form.reset();
                    self.load_departments();
                }
                UiMessage::DepartmentDeleted(id) => {
                    self.departments.retain(|d| d.id != id);
                    self.success_message = Some("Department deleted".to_string());
                    self.log_success("Department deleted");
                }
                UiMessage::EmployeeSaved(emp) => {
                    self.success_message = Some(format!("Employee '{name}' saved", name = emp.full_name));
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
                    self.is_loading = false;
                    self.success_message = Some(format!("Exported to {path}"));
                    self.log_success(format!("Export completed: {path}"));
                }
                UiMessage::ExportFailed(e) => {
                    self.is_loading = false;
                    self.error_message = Some(e.clone());
                    self.log_error(e);
                }
                UiMessage::DeviceTestResult(ok) => {
                    self.device_test_status = Some(ok);
                    if ok {
                        self.device_status = DeviceStatus::Connected;
                        self.log_success("Device connection successful");
                    } else {
                        self.device_status = DeviceStatus::Error;
                        self.log_error("Device connection failed");
                    }
                }
                UiMessage::DatabaseTestResult(ok) => {
                    self.database_test_status = Some(ok);
                    if ok {
                        self.log_success("Database connection successful");
                    } else {
                        self.log_error("Database connection failed");
                    }
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
                    ui.colored_label(color, format!("Device: {text}"));

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
                                ui.colored_label(colors::ERROR, format!("Failed: {e}"));
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
                DeleteTarget::Department(_, name) => ("Delete Department", format!("Delete department '{name}'?")),
                DeleteTarget::Employee(_, name) => ("Delete Employee", format!("Delete employee '{name}'?")),
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
                    self.log_info(format!("Deleting department: {name}"));
                    self.delete_department(id);
                }
                DeleteTarget::Employee(id, name) => {
                    self.log_info(format!("Deleting employee: {name}"));
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
                if reports_panel::show(self, ui) {
                    self.current_panel = Panel::Dashboard;
                }
            }
            Panel::Settings => {
                if settings_panel::show(self, ui) {
                    self.current_panel = Panel::Dashboard;
                }
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
