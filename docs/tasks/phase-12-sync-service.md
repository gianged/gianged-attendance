# Phase 12: Sync Service

## Objective

Implement sync orchestration between device and database.

---

## Tasks

### 12.1 Create Sync Module

**`src/sync.rs`**

```rust
use crate::client::ZkClient;
use crate::config::AppConfig;
use crate::db::attendance;
use crate::error::Result;
use chrono::{Duration, Utc};
use sqlx::PgPool;

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub downloaded: usize,
    pub inserted: usize,
    pub skipped: usize,
    pub duration_secs: f64,
}

impl SyncResult {
    /// Get summary message
    pub fn summary(&self) -> String {
        format!(
            "Downloaded: {}, Inserted: {}, Skipped: {} (took {:.1}s)",
            self.downloaded, self.inserted, self.skipped, self.duration_secs
        )
    }
}

/// Sync service for orchestrating data transfer
pub struct SyncService {
    config: AppConfig,
    pool: PgPool,
}

impl SyncService {
    /// Create a new sync service
    pub fn new(config: AppConfig, pool: PgPool) -> Self {
        Self { config, pool }
    }

    /// Perform a sync operation
    pub async fn sync(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        // Create client and login
        let mut client = ZkClient::new(&self.config.device.url);
        client
            .login(&self.config.device.username, &self.config.device.password)
            .await?;

        // Calculate date range
        let end_date = Utc::now().date_naive();
        let start_date = end_date - Duration::days(self.config.sync.days as i64);

        // Build user ID list
        let user_ids: Vec<i32> = (1..=self.config.sync.max_user_id).collect();

        // Download attendance data
        let records = client
            .download_attendance(start_date, end_date, &user_ids)
            .await?;

        let downloaded = records.len();

        // Insert into database
        let inserted = attendance::insert_batch(&self.pool, &records).await?;
        let skipped = downloaded - inserted;

        let duration_secs = start.elapsed().as_secs_f64();

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// Perform sync with progress callback
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
        let start_date = end_date - Duration::days(self.config.sync.days as i64);
        let user_ids: Vec<i32> = (1..=self.config.sync.max_user_id).collect();

        on_progress(0.3, "Downloading attendance data...");
        let records = client
            .download_attendance(start_date, end_date, &user_ids)
            .await?;

        let downloaded = records.len();
        on_progress(0.6, &format!("Downloaded {} records", downloaded));

        on_progress(0.7, "Inserting into database...");
        let inserted = attendance::insert_batch(&self.pool, &records).await?;
        let skipped = downloaded - inserted;

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

    /// Test device connection
    pub async fn test_device_connection(&self) -> Result<bool> {
        let client = ZkClient::new(&self.config.device.url);
        client.test_connection().await
    }

    /// Test device login
    pub async fn test_device_login(&self) -> Result<bool> {
        let mut client = ZkClient::new(&self.config.device.url);
        match client
            .login(&self.config.device.username, &self.config.device.password)
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
```

### 12.2 Background Sync Task

For running sync in background with tokio:

```rust
use std::sync::Arc;
use tokio::sync::mpsc;

/// Message sent from sync task to UI
pub enum SyncMessage {
    Progress(f32, String),
    Completed(SyncResult),
    Failed(String),
}

/// Run sync in background and report progress via channel
pub async fn run_sync_background(
    config: AppConfig,
    pool: PgPool,
    tx: mpsc::UnboundedSender<SyncMessage>,
) {
    let service = SyncService::new(config, pool);

    let result = service
        .sync_with_progress(|progress, message| {
            let _ = tx.send(SyncMessage::Progress(progress, message.to_string()));
        })
        .await;

    match result {
        Ok(sync_result) => {
            let _ = tx.send(SyncMessage::Completed(sync_result));
        }
        Err(e) => {
            let _ = tx.send(SyncMessage::Failed(e.to_string()));
        }
    }
}
```

---

## Deliverables

- [x] SyncResult struct with summary
- [x] SyncService struct
- [x] sync() basic function
- [x] sync_with_progress() with callback
- [x] test_device_connection()
- [x] test_device_login()
- [x] Background sync task
- [x] SyncMessage enum for UI communication
