//! Department DTOs for create and update operations.

use serde::{Deserialize, Serialize};

/// DTO for creating a department.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDepartment {
    pub name: String,
    pub parent_id: Option<i32>,
    pub display_order: i32,
}

/// DTO for updating a department.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateDepartment {
    pub name: Option<String>,
    pub parent_id: Option<Option<i32>>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
}
