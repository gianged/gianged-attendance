//! Employee DTOs for create and update operations.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// DTO for creating an employee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmployee {
    pub employee_code: String,
    pub full_name: String,
    pub department_id: Option<i32>,
    pub scanner_uid: Option<i32>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub start_date: NaiveDate,
}

/// DTO for updating an employee.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateEmployee {
    pub employee_code: Option<String>,
    pub full_name: Option<String>,
    pub department_id: Option<Option<i32>>,
    pub scanner_uid: Option<Option<i32>>,
    pub gender: Option<Option<String>>,
    pub birth_date: Option<Option<NaiveDate>>,
    pub start_date: Option<NaiveDate>,
    pub is_active: Option<bool>,
}
