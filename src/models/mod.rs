//! Data models for departments, employees, and attendance logs.

pub mod attendance;
pub mod department;
pub mod employee;

pub use attendance::{CreateAttendanceLog, DailyAttendance, verify_type};
pub use department::{CreateDepartment, UpdateDepartment};
pub use employee::{CreateEmployee, UpdateEmployee};
