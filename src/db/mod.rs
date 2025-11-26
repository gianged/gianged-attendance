//! Database connection pool and operations.

use sea_orm::{Database, DatabaseConnection, DbErr};

/// Create a database connection pool.
pub async fn create_pool(conn_str: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(conn_str).await
}

/// Test database connection.
#[allow(dead_code)]
pub async fn test_connection(conn: &DatabaseConnection) -> Result<(), DbErr> {
    conn.ping().await
}

/// Test connection string without keeping the connection.
#[allow(dead_code)]
pub async fn test_connection_string(conn_str: &str) -> Result<(), String> {
    let conn = Database::connect(conn_str).await.map_err(|e| e.to_string())?;

    conn.ping().await.map_err(|e| e.to_string())
}
