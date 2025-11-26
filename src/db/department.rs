//! Department repository with CRUD operations.

use crate::entities::{departments, prelude::*};
use crate::models::department::{CreateDepartment, UpdateDepartment};
use sea_orm::*;

/// List all departments ordered by display_order and name.
pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<departments::Model>, DbErr> {
    Departments::find()
        .order_by_asc(departments::Column::DisplayOrder)
        .order_by_asc(departments::Column::Name)
        .all(db)
        .await
}

/// List only active departments.
pub async fn list_active(db: &DatabaseConnection) -> Result<Vec<departments::Model>, DbErr> {
    Departments::find()
        .filter(departments::Column::IsActive.eq(true))
        .order_by_asc(departments::Column::DisplayOrder)
        .order_by_asc(departments::Column::Name)
        .all(db)
        .await
}

/// Get department by ID.
pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<departments::Model>, DbErr> {
    Departments::find_by_id(id).one(db).await
}

/// Create a new department.
pub async fn create(db: &DatabaseConnection, data: CreateDepartment) -> Result<departments::Model, DbErr> {
    let model = departments::ActiveModel {
        name: Set(data.name),
        parent_id: Set(data.parent_id),
        display_order: Set(data.display_order),
        ..Default::default()
    };
    model.insert(db).await
}

/// Update an existing department.
pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    data: UpdateDepartment,
) -> Result<Option<departments::Model>, DbErr> {
    let existing = Departments::find_by_id(id).one(db).await?;

    match existing {
        Some(model) => {
            let mut active: departments::ActiveModel = model.into();

            if let Some(name) = data.name {
                active.name = Set(name);
            }
            if let Some(parent_id) = data.parent_id {
                active.parent_id = Set(parent_id);
            }
            if let Some(display_order) = data.display_order {
                active.display_order = Set(display_order);
            }
            if let Some(is_active) = data.is_active {
                active.is_active = Set(is_active);
            }

            let updated = active.update(db).await?;
            Ok(Some(updated))
        }
        None => Ok(None),
    }
}

/// Delete a department by ID.
pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<bool, DbErr> {
    let result = Departments::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}

/// Check if department name exists (for validation).
pub async fn name_exists(db: &DatabaseConnection, name: &str, exclude_id: Option<i32>) -> Result<bool, DbErr> {
    let mut query = Departments::find().filter(departments::Column::Name.eq(name));

    if let Some(id) = exclude_id {
        query = query.filter(departments::Column::Id.ne(id));
    }

    let count = query.count(db).await?;
    Ok(count > 0)
}

/// Get child departments.
pub async fn get_children(db: &DatabaseConnection, parent_id: i32) -> Result<Vec<departments::Model>, DbErr> {
    Departments::find()
        .filter(departments::Column::ParentId.eq(parent_id))
        .order_by_asc(departments::Column::DisplayOrder)
        .order_by_asc(departments::Column::Name)
        .all(db)
        .await
}
