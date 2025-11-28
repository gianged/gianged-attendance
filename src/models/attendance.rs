//! Attendance DTOs and view models.

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

/// DTO for creating an attendance log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttendanceLog {
    pub scanner_uid: i32,
    pub check_time: DateTime<Utc>,
    pub verify_type: i32,
    pub status: i32,
    pub source: String,
}

/// Daily attendance summary from v_daily_attendance view.
#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct DailyAttendance {
    pub employee_id: i32,
    pub employee_code: String,
    pub full_name: String,
    pub department_id: Option<i32>,
    pub department_name: Option<String>,
    pub work_date: NaiveDate,
    pub first_check: DateTime<Utc>,
    pub last_check: DateTime<Utc>,
    pub check_count: i64,
    pub work_hours: Option<f64>,
}

/// Attendance detail from v_attendance_details view.
/// Contains individual check records with employee info.
#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct AttendanceDetail {
    pub id: i64,
    pub scanner_uid: i32,
    pub employee_id: Option<i32>,
    pub employee_code: Option<String>,
    pub full_name: Option<String>,
    pub department_id: Option<i32>,
    pub department_name: Option<String>,
    pub check_time: DateTime<Utc>,
    pub verify_type: i32,
    pub verify_type_name: String,
    pub source: String,
}

/// Verify type constants matching database CHECK constraint.
pub mod verify_type {
    /// Fingerprint verification (device code: 2).
    pub const FINGERPRINT: i32 = 2;
    /// Card verification (device code: 101).
    pub const CARD: i32 = 101;

    /// Get human-readable name for verify type code.
    pub fn name(code: i32) -> &'static str {
        match code {
            FINGERPRINT => "fingerprint",
            CARD => "card",
            _ => "unknown",
        }
    }
}

impl DailyAttendance {
    /// Calculate work duration in hours from first and last check times.
    pub fn calculate_work_hours(&self) -> f64 {
        let duration = self.last_check - self.first_check;
        duration.num_minutes() as f64 / 60.0
    }
}
