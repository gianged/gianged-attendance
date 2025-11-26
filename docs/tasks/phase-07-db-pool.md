# Phase 07: Database Connection

## Objective

Set up PostgreSQL connection using SeaORM.

---

## Tasks

### 7.1 Create DB Directory

```
src/db/
├── mod.rs
└── connection.rs
```

### 7.2 Database Connection

**`src/db/connection.rs`**

```rust
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;
use tracing::log::LevelFilter;

/// Create a new database connection
pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(5)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(true)
        .sqlx_logging_level(LevelFilter::Debug);

    Database::connect(opt).await
}

/// Test database connection
pub async fn test_connection(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::ConnectionTrait;
    db.execute_unprepared("SELECT 1").await?;
    Ok(())
}

/// Get database version
pub async fn get_version(db: &DatabaseConnection) -> Result<String, DbErr> {
    use sea_orm::{ConnectionTrait, Statement};

    let result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT version()".to_owned(),
        ))
        .await?;

    match result {
        Some(row) => {
            use sea_orm::QueryResult;
            let version: String = row.try_get("", "version")?;
            Ok(version)
        }
        None => Ok("Unknown".to_owned()),
    }
}

/// Get record counts for all tables
pub async fn get_table_counts(db: &DatabaseConnection) -> Result<TableCounts, DbErr> {
    use crate::entities::prelude::*;
    use sea_orm::EntityTrait;

    let departments = Departments::find().count(db).await?;
    let employees = Employees::find().count(db).await?;
    let attendance_logs = AttendanceLogs::find().count(db).await?;

    Ok(TableCounts {
        departments,
        employees,
        attendance_logs,
    })
}

/// Table record counts
#[derive(Debug, Clone)]
pub struct TableCounts {
    pub departments: u64,
    pub employees: u64,
    pub attendance_logs: u64,
}
```

### 7.3 Module Export

**`src/db/mod.rs`**

```rust
pub mod connection;

pub use connection::{connect, test_connection, get_version, get_table_counts, TableCounts};
```

### 7.4 Test Connection

Add a test in main.rs temporarily:

```rust
use sea_orm::DatabaseConnection;

#[tokio::main]
async fn main() {
    let database_url = "postgres://postgres:password@localhost:5432/gianged_attendance";

    match db::connect(database_url).await {
        Ok(db) => {
            println!("Connected to database!");

            if let Ok(version) = db::get_version(&db).await {
                println!("PostgreSQL: {}", version);
            }

            if let Ok(counts) = db::get_table_counts(&db).await {
                println!("Departments: {}", counts.departments);
                println!("Employees: {}", counts.employees);
                println!("Attendance: {}", counts.attendance_logs);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
        }
    }
}
```

---

## Deliverables

- [ ] Database connection with ConnectOptions
- [ ] Connection test function
- [ ] Version query
- [ ] Table counts using entity queries
- [ ] Successful connection test
