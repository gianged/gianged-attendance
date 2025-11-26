//! Database connection pool and utility functions.

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, PaginatorTrait, Statement};
use std::time::Duration;
use tracing::log::LevelFilter;

/// Create a new database connection with configured pool settings.
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

/// Test database connection by executing a simple query.
pub async fn test_connection(db: &DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared("SELECT 1").await?;
    Ok(())
}

/// Get PostgreSQL version string.
pub async fn get_version(db: &DatabaseConnection) -> Result<String, DbErr> {
    let result = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT version()".to_owned(),
        ))
        .await?;

    match result {
        Some(row) => {
            let version: String = row.try_get("", "version")?;
            Ok(version)
        }
        None => Ok("Unknown".to_owned()),
    }
}

/// Get record counts for all tables.
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

/// Table record counts.
#[derive(Debug, Clone)]
pub struct TableCounts {
    pub departments: u64,
    pub employees: u64,
    pub attendance_logs: u64,
}
