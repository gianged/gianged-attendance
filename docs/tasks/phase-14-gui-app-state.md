# Phase 14: GUI Application State

## Objective

Define application state and message passing for the egui desktop app.

---

## Tasks

### 14.1 Create App Module

**`src/app.rs`**

```rust
use crate::config::AppConfig;
use crate::models::{
    attendance::DailyAttendance,
    department::Department,
    employee::Employee,
};
use crate::sync::SyncResult;
use chrono::NaiveDate;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// Current panel/view in the application
#[derive(Default, Clone, Copy, PartialEq, Eq)]
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

/// Messages from async tasks to UI
pub enum UiMessage {
    // Data loading
    DepartmentsLoaded(Vec<Department>),
    EmployeesLoaded(Vec<Employee>),
    AttendanceLoaded(Vec<DailyAttendance>),
    LoadError(String),

    // Sync
    SyncProgress(f32, String),
    SyncCompleted(SyncResult),
    SyncFailed(String),

    // CRUD operations
    DepartmentSaved(Department),
    DepartmentDeleted(i32),
    EmployeeSaved(Employee),
    EmployeeDeleted(i32),
    OperationFailed(String),

    // Export
    ExportCompleted(String),
    ExportFailed(String),

    // Connection tests
    DeviceTestResult(bool),
    DatabaseTestResult(bool),
}

/// Main application state
pub struct App {
    // Runtime and database
    pub rt: Runtime,
    pub pool: Arc<PgPool>,

    // Message channel
    pub tx: mpsc::UnboundedSender<UiMessage>,
    pub rx: mpsc::UnboundedReceiver<UiMessage>,

    // Navigation
    pub current_panel: Panel,

    // Data
    pub departments: Vec<Department>,
    pub employees: Vec<Employee>,
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
}

/// Form state for department CRUD
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
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn edit(dept: &Department) -> Self {
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

/// Form state for employee CRUD
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
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn edit(emp: &Employee) -> Self {
        Self {
            id: Some(emp.id),
            employee_code: emp.employee_code.clone(),
            full_name: emp.full_name.clone(),
            department_id: emp.department_id,
            device_uid: emp.device_uid.map(|u| u.to_string()).unwrap_or_default(),
            gender: emp.gender.clone(),
            birth_date: emp.birth_date,
            start_date: Some(emp.start_date),
            end_date: emp.end_date,
            is_active: emp.is_active,
            is_open: true,
            is_editing: true,
        }
    }
}

/// Filter state for reports
#[derive(Clone)]
pub struct ReportFilter {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub department_id: Option<i32>,
    pub employee_id: Option<i32>,
}

impl Default for ReportFilter {
    fn default() -> Self {
        let today = chrono::Local::now().date_naive();
        Self {
            start_date: today - chrono::Duration::days(30),
            end_date: today,
            department_id: None,
            employee_id: None,
        }
    }
}

/// Log entry for display
#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub message: String,
    pub level: LogLevel,
}

#[derive(Clone, Copy)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Target for delete confirmation
#[derive(Clone)]
pub enum DeleteTarget {
    Department(i32, String),
    Employee(i32, String),
}
```

### 14.2 App Implementation

```rust
impl App {
    pub fn new(pool: Arc<PgPool>, config: AppConfig, rt: Runtime) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut app = Self {
            rt,
            pool,
            tx,
            rx,
            current_panel: Panel::Dashboard,
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
            log_messages: Vec::new(),
            config,
            config_modified: false,
            employee_search: String::new(),
            employee_dept_filter: None,
            show_delete_confirm: false,
            delete_target: None,
            error_message: None,
            success_message: None,
        };

        // Load initial data
        app.load_departments();
        app.load_employees();

        app
    }

    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.log_messages.push(LogEntry {
            timestamp: chrono::Local::now(),
            message: message.into(),
            level,
        });

        // Keep only last 100 messages
        if self.log_messages.len() > 100 {
            self.log_messages.remove(0);
        }
    }

    pub fn log_info(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Info, message);
    }

    pub fn log_success(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Success, message);
    }

    pub fn log_error(&mut self, message: impl Into<String>) {
        self.log(LogLevel::Error, message);
    }
}
```

---

## Deliverables

- [ ] Panel enum for navigation
- [ ] UiMessage enum for async communication
- [ ] App struct with all state
- [ ] DepartmentForm struct
- [ ] EmployeeForm struct
- [ ] ReportFilter struct
- [ ] LogEntry and LogLevel
- [ ] DeleteTarget enum
- [ ] App::new() constructor
- [ ] Logging helper methods
