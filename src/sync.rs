//! Sync service orchestration.

use crate::client::ZkClient;
use crate::config::AppConfig;
use crate::db::attendance;
use crate::error::Result;
use crate::models::attendance::CreateAttendanceLog;
use crate::ui::app::SyncProgress;
use crate::zk::{AttendanceRecord as ZkAttendance, ZkTcpClient};
use chrono::{Local, TimeDelta, Utc};
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc;
use tracing::info;

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub downloaded: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub duration_secs: f64,
}

impl SyncResult {
    /// Get summary message.
    pub fn summary(&self) -> String {
        format!(
            "Downloaded: {}, Inserted: {}, Skipped: {} (took {:.1}s)",
            self.downloaded, self.inserted, self.skipped, self.duration_secs
        )
    }
}

/// Sync service for orchestrating data transfer.
pub struct SyncService {
    config: AppConfig,
    db: DatabaseConnection,
}

impl SyncService {
    /// Create a new sync service.
    pub fn new(config: AppConfig, db: DatabaseConnection) -> Self {
        Self { config, db }
    }

    /// Perform a sync operation.
    pub async fn sync(&self) -> Result<SyncResult> {
        if self.config.device.use_tcp() {
            self.sync_via_tcp().await
        } else {
            self.sync_via_http().await
        }
    }

    /// Sync via TCP protocol (reads from flash storage).
    async fn sync_via_tcp(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        let device_ip = self.config.device.device_ip().to_string();

        info!("Starting TCP sync from {device_ip}:4370");

        // Run blocking TCP client in spawn_blocking
        let records = tokio::task::spawn_blocking(move || {
            let addr = format!("{device_ip}:4370");
            let mut client = ZkTcpClient::connect(&addr)?;
            client.get_attendance()
        })
        .await
        .map_err(|e| crate::error::AppError::parse(format!("Task join error: {e}")))??;

        let downloaded = records.len();

        // Convert ZK records to CreateAttendanceLog
        let logs: Vec<CreateAttendanceLog> = records.into_iter().map(convert_zk_record).collect();

        // Insert into database
        let inserted = attendance::insert_batch(&self.db, &logs).await?;
        let skipped = downloaded.saturating_sub(inserted);

        let duration_secs = start.elapsed().as_secs_f64();

        info!("TCP sync complete: {downloaded} downloaded, {inserted} inserted");

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// Sync via HTTP protocol (legacy, limited buffer).
    async fn sync_via_http(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        // Create client and login
        let mut client = ZkClient::new(&self.config.device.url);
        client
            .login(&self.config.device.username, &self.config.device.password)
            .await?;

        // Calculate date range
        let end_date = Utc::now().date_naive();
        let start_date = end_date - TimeDelta::days(i64::from(self.config.sync.days));

        // Build user ID list
        let user_ids: Vec<i32> = (1..=self.config.sync.max_user_id).collect();

        // Download attendance data
        let records = client.download_attendance(start_date, end_date, &user_ids).await?;

        let downloaded = records.len();

        // Insert into database
        let inserted = attendance::insert_batch(&self.db, &records).await?;
        let skipped = downloaded.saturating_sub(inserted);

        let duration_secs = start.elapsed().as_secs_f64();

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// Perform sync with progress callback.
    pub async fn sync_with_progress<F>(&self, on_progress: F) -> Result<SyncResult>
    where
        F: FnMut(f32, &str),
    {
        if self.config.device.use_tcp() {
            self.sync_via_tcp_with_progress(on_progress).await
        } else {
            self.sync_via_http_with_progress(on_progress).await
        }
    }

    /// TCP sync with progress callback.
    async fn sync_via_tcp_with_progress<F>(&self, mut on_progress: F) -> Result<SyncResult>
    where
        F: FnMut(f32, &str),
    {
        let start = std::time::Instant::now();
        let device_ip = self.config.device.device_ip().to_string();

        on_progress(0.0, "Connecting to device (TCP)...");

        // Run blocking TCP client in spawn_blocking
        let records = tokio::task::spawn_blocking(move || {
            let addr = format!("{device_ip}:4370");
            let mut client = ZkTcpClient::connect(&addr)?;
            client.get_attendance()
        })
        .await
        .map_err(|e| crate::error::AppError::parse(format!("Task join error: {e}")))??;

        let downloaded = records.len();
        on_progress(0.6, &format!("Downloaded {downloaded} records"));

        // Convert ZK records to CreateAttendanceLog
        let logs: Vec<CreateAttendanceLog> = records.into_iter().map(convert_zk_record).collect();

        on_progress(0.7, "Inserting into database...");
        let inserted = attendance::insert_batch(&self.db, &logs).await?;
        let skipped = downloaded.saturating_sub(inserted);

        on_progress(0.9, "Finalizing...");

        let duration_secs = start.elapsed().as_secs_f64();

        on_progress(1.0, &format!("Done! Inserted {inserted} new records"));

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// HTTP sync with progress callback.
    async fn sync_via_http_with_progress<F>(&self, mut on_progress: F) -> Result<SyncResult>
    where
        F: FnMut(f32, &str),
    {
        let start = std::time::Instant::now();

        on_progress(0.0, "Connecting to device (HTTP)...");

        let mut client = ZkClient::new(&self.config.device.url);

        on_progress(0.1, "Logging in...");
        client
            .login(&self.config.device.username, &self.config.device.password)
            .await?;

        on_progress(0.2, "Preparing download...");

        let end_date = Utc::now().date_naive();
        let start_date = end_date - TimeDelta::days(i64::from(self.config.sync.days));
        let user_ids: Vec<i32> = (1..=self.config.sync.max_user_id).collect();

        on_progress(0.3, "Downloading attendance data...");
        let records = client.download_attendance(start_date, end_date, &user_ids).await?;

        let downloaded = records.len();
        on_progress(0.6, &format!("Downloaded {downloaded} records"));

        on_progress(0.7, "Inserting into database...");
        let inserted = attendance::insert_batch(&self.db, &records).await?;
        let skipped = downloaded.saturating_sub(inserted);

        on_progress(0.9, "Finalizing...");

        let duration_secs = start.elapsed().as_secs_f64();

        on_progress(1.0, &format!("Done! Inserted {inserted} new records"));

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// Test device connection.
    pub async fn test_device_connection(&self) -> Result<bool> {
        if self.config.device.use_tcp() {
            let device_ip = self.config.device.device_ip().to_string();
            let result = tokio::task::spawn_blocking(move || {
                let addr = format!("{device_ip}:4370");
                ZkTcpClient::connect(&addr).map(|_| true)
            })
            .await
            .map_err(|e| crate::error::AppError::parse(format!("Task join error: {e}")))?;
            Ok(result.unwrap_or(false))
        } else {
            let client = ZkClient::new(&self.config.device.url);
            client.test_connection().await
        }
    }

    /// Test device login (HTTP only, TCP has no login).
    pub async fn test_device_login(&self) -> Result<bool> {
        if self.config.device.use_tcp() {
            // TCP protocol doesn't use login, connection test is sufficient
            self.test_device_connection().await
        } else {
            let mut client = ZkClient::new(&self.config.device.url);
            match client
                .login(&self.config.device.username, &self.config.device.password)
                .await
            {
                Ok(()) => Ok(true),
                Err(_) => Ok(false),
            }
        }
    }
}

/// Run sync in background and report progress via channel.
pub async fn run_sync_background(config: AppConfig, db: DatabaseConnection, tx: mpsc::UnboundedSender<SyncProgress>) {
    let service = SyncService::new(config, db);

    let result = service
        .sync_with_progress(|progress, message| {
            let _ = tx.send(SyncProgress::Progress {
                percent: progress,
                message: message.to_string(),
            });
        })
        .await;

    match result {
        Ok(sync_result) => {
            let _ = tx.send(SyncProgress::Completed {
                records: sync_result.inserted as u32,
                timestamp: Local::now(),
            });
        }
        Err(e) => {
            let _ = tx.send(SyncProgress::Error(e.to_string()));
        }
    }
}

/// Convert ZK attendance record to database model.
fn convert_zk_record(record: ZkAttendance) -> CreateAttendanceLog {
    CreateAttendanceLog {
        scanner_uid: record.user_id as i32,
        check_time: record.timestamp.to_utc(), // Convert local time to UTC for storage
        verify_type: 2,                        // Default to fingerprint (TCP doesn't provide this)
        status: 0,
        source: "device".to_string(),
    }
}
