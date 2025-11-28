//! Sync service orchestration.

use crate::client::ZkClient;
use crate::config::AppConfig;
use crate::db::attendance;
use crate::error::Result;
use crate::ui::main_app::SyncProgress;
use chrono::{Local, TimeDelta, Utc};
use sea_orm::DatabaseConnection;
use std::sync::mpsc;

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
    pub async fn sync_with_progress<F>(&self, mut on_progress: F) -> Result<SyncResult>
    where
        F: FnMut(f32, &str),
    {
        let start = std::time::Instant::now();

        on_progress(0.0, "Connecting to device...");

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
        on_progress(0.6, &format!("Downloaded {} records", downloaded));

        on_progress(0.7, "Inserting into database...");
        let inserted = attendance::insert_batch(&self.db, &records).await?;
        let skipped = downloaded.saturating_sub(inserted);

        on_progress(0.9, "Finalizing...");

        let duration_secs = start.elapsed().as_secs_f64();

        on_progress(1.0, &format!("Done! Inserted {} new records", inserted));

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// Test device connection.
    pub async fn test_device_connection(&self) -> Result<bool> {
        let client = ZkClient::new(&self.config.device.url);
        client.test_connection().await
    }

    /// Test device login.
    pub async fn test_device_login(&self) -> Result<bool> {
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

/// Run sync in background and report progress via channel.
pub async fn run_sync_background(config: AppConfig, db: DatabaseConnection, tx: mpsc::Sender<SyncProgress>) {
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
