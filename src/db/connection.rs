//! Database connection pool and utility functions.

use sea_orm::sqlx::Executor;
use sea_orm::sqlx::postgres::PgPoolOptions;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, PaginatorTrait, SqlxPostgresConnector, Statement};
use std::time::Duration;

/// Create a new database connection with configured pool settings.
/// Uses after_connect callback to set search_path on each connection.
pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    // Build sqlx pool with after_connect callback
    let sqlx_pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                // Set search_path for each new connection
                conn.execute("SET search_path TO app, system, public").await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .map_err(|e| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(e)))?;

    // Convert sqlx pool to SeaORM DatabaseConnection
    Ok(SqlxPostgresConnector::from_sqlx_postgres_pool(sqlx_pool))
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
