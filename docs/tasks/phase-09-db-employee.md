# Phase 09: Employee Repository

## Objective

Implement CRUD operations for employees using SeaORM.

---

## Tasks

### 9.1 Create Repository

**`src/db/employee.rs`**

```rust
use crate::entities::{employees, prelude::*};
use crate::models::employee::{CreateEmployee, UpdateEmployee};
use sea_orm::*;

/// List all employees ordered by employee_code
pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<employees::Model>, DbErr> {
    Employees::find()
        .order_by_asc(employees::Column::EmployeeCode)
        .all(db)
        .await
}

/// List employees by department
pub async fn list_by_department(
    db: &DatabaseConnection,
    department_id: i32,
) -> Result<Vec<employees::Model>, DbErr> {
    Employees::find()
        .filter(employees::Column::DepartmentId.eq(department_id))
        .order_by_asc(employees::Column::EmployeeCode)
        .all(db)
        .await
}

/// List only active employees
pub async fn list_active(db: &DatabaseConnection) -> Result<Vec<employees::Model>, DbErr> {
    Employees::find()
        .filter(employees::Column::IsActive.eq(true))
        .order_by_asc(employees::Column::EmployeeCode)
        .all(db)
        .await
}

/// Search employees by code or name
pub async fn search(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<employees::Model>, DbErr> {
    let pattern = format!("%{}%", query);
    Employees::find()
        .filter(
            Condition::any()
                .add(employees::Column::EmployeeCode.like(&pattern))
                .add(employees::Column::FullName.like(&pattern)),
        )
        .order_by_asc(employees::Column::EmployeeCode)
        .all(db)
        .await
}

/// Get employee by ID
pub async fn get_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<employees::Model>, DbErr> {
    Employees::find_by_id(id).one(db).await
}

/// Get employee by device UID
pub async fn get_by_device_uid(
    db: &DatabaseConnection,
    device_uid: i32,
) -> Result<Option<employees::Model>, DbErr> {
    Employees::find()
        .filter(employees::Column::DeviceUid.eq(device_uid))
        .one(db)
        .await
}

/// Get employee by employee code
pub async fn get_by_code(
    db: &DatabaseConnection,
    code: &str,
) -> Result<Option<employees::Model>, DbErr> {
    Employees::find()
        .filter(employees::Column::EmployeeCode.eq(code))
        .one(db)
        .await
}

/// Create a new employee
pub async fn create(
    db: &DatabaseConnection,
    data: CreateEmployee,
) -> Result<employees::Model, DbErr> {
    let model = employees::ActiveModel {
        employee_code: Set(data.employee_code),
        full_name: Set(data.full_name),
        department_id: Set(data.department_id),
        device_uid: Set(data.device_uid),
        gender: Set(data.gender),
        birth_date: Set(data.birth_date),
        start_date: Set(data.start_date),
        ..Default::default()
    };
    model.insert(db).await
}

/// Update an existing employee
pub async fn update(
    db: &DatabaseConnection,
    id: i32,
    data: UpdateEmployee,
) -> Result<Option<employees::Model>, DbErr> {
    let existing = Employees::find_by_id(id).one(db).await?;

    match existing {
        Some(model) => {
            let mut active: employees::ActiveModel = model.into();

            if let Some(employee_code) = data.employee_code {
                active.employee_code = Set(employee_code);
            }
            if let Some(full_name) = data.full_name {
                active.full_name = Set(full_name);
            }
            if let Some(department_id) = data.department_id {
                active.department_id = Set(department_id);
            }
            if let Some(device_uid) = data.device_uid {
                active.device_uid = Set(device_uid);
            }
            if let Some(gender) = data.gender {
                active.gender = Set(gender);
            }
            if let Some(birth_date) = data.birth_date {
                active.birth_date = Set(birth_date);
            }
            if let Some(start_date) = data.start_date {
                active.start_date = Set(start_date);
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

/// Delete an employee by ID
pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<bool, DbErr> {
    let result = Employees::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}

/// Check if employee code exists
pub async fn code_exists(
    db: &DatabaseConnection,
    code: &str,
    exclude_id: Option<i32>,
) -> Result<bool, DbErr> {
    let mut query = Employees::find().filter(employees::Column::EmployeeCode.eq(code));

    if let Some(id) = exclude_id {
        query = query.filter(employees::Column::Id.ne(id));
    }

    let count = query.count(db).await?;
    Ok(count > 0)
}

/// Check if device UID is already assigned
pub async fn device_uid_exists(
    db: &DatabaseConnection,
    device_uid: i32,
    exclude_id: Option<i32>,
) -> Result<bool, DbErr> {
    let mut query = Employees::find().filter(employees::Column::DeviceUid.eq(device_uid));

    if let Some(id) = exclude_id {
        query = query.filter(employees::Column::Id.ne(id));
    }

    let count = query.count(db).await?;
    Ok(count > 0)
}
```

### 9.2 Update Module Export

**`src/db/mod.rs`**

```rust
pub mod connection;
pub mod department;
pub mod employee;

pub use connection::{connect, test_connection, get_version, get_table_counts, TableCounts};
```

---

## Deliverables

- [ ] list_all function
- [ ] list_by_department function
- [ ] list_active function
- [ ] search function
- [ ] get_by_id function
- [ ] get_by_device_uid function
- [ ] get_by_code function
- [ ] create function
- [ ] update function
- [ ] delete function
- [ ] code_exists validation
- [ ] device_uid_exists validation
