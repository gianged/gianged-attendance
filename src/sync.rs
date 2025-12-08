//! Sync service orchestration.

use crate::config::AppConfig;
use crate::db::attendance;
use crate::error::{AppError, Result};
use crate::ui::app::SyncProgress;
use crate::zk_tcp::ZkTcpClient;
use chrono::Local;
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc;

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

    /// Extract IP address from device URL.
    fn extract_ip_from_url(url: &str) -> Result<String> {
        // Remove protocol prefix
        let without_protocol = url.trim_start_matches("http://").trim_start_matches("https://");

        // Extract host (before port or path)
        let ip = without_protocol
            .split(':')
            .next()
            .unwrap_or(without_protocol)
            .split('/')
            .next()
            .unwrap_or(without_protocol);

        if ip.is_empty() {
            return Err(AppError::config("Invalid device URL: cannot extract IP"));
        }

        Ok(ip.to_string())
    }

    /// Perform a sync operation using TCP protocol.
    pub async fn sync(&self) -> Result<SyncResult> {
        let start = std::time::Instant::now();

        // Extract IP from URL
        let ip = Self::extract_ip_from_url(&self.config.device.url)?;

        // Create TCP client and connect
        let mut client = ZkTcpClient::new(&ip, self.config.device.tcp_port, self.config.device.tcp_timeout_secs);
        client.connect().await?;

        // Download attendance data
        let records = client.download_attendance().await?;

        // Disconnect
        client.disconnect().await?;

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

    /// Perform sync with progress callback using TCP protocol.
    pub async fn sync_with_progress<F>(&self, mut on_progress: F) -> Result<SyncResult>
    where
        F: FnMut(f32, &str),
    {
        let start = std::time::Instant::now();

        on_progress(0.0, "Connecting to device (TCP)...");

        // Extract IP from URL
        let ip = Self::extract_ip_from_url(&self.config.device.url)?;

        // Create TCP client
        let mut client = ZkTcpClient::new(&ip, self.config.device.tcp_port, self.config.device.tcp_timeout_secs);

        on_progress(0.1, "Establishing TCP session...");
        client.connect().await?;

        on_progress(0.3, "Downloading attendance data...");
        let records = client.download_attendance().await?;

        let downloaded = records.len();
        on_progress(0.6, &format!("Downloaded {downloaded} records"));

        on_progress(0.7, "Inserting into database...");
        let inserted = attendance::insert_batch(&self.db, &records).await?;
        let skipped = downloaded.saturating_sub(inserted);

        on_progress(0.9, "Closing connection...");
        client.disconnect().await?;

        let duration_secs = start.elapsed().as_secs_f64();

        on_progress(1.0, &format!("Done! Inserted {inserted} new records"));

        Ok(SyncResult {
            downloaded,
            inserted,
            skipped,
            duration_secs,
        })
    }

    /// Test device TCP connection.
    pub async fn test_device_connection(&self) -> Result<bool> {
        let ip = Self::extract_ip_from_url(&self.config.device.url)?;
        let mut client = ZkTcpClient::new(&ip, self.config.device.tcp_port, self.config.device.tcp_timeout_secs);

        match client.connect().await {
            Ok(()) => {
                let _ = client.disconnect().await;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    /// Get attendance record count from device via TCP.
    pub async fn get_device_record_count(&self) -> Result<u32> {
        let ip = Self::extract_ip_from_url(&self.config.device.url)?;
        let mut client = ZkTcpClient::new(&ip, self.config.device.tcp_port, self.config.device.tcp_timeout_secs);

        client.connect().await?;
        let count = client.get_attendance_count().await?;
        client.disconnect().await?;

        Ok(count)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ip_from_url_http() {
        let ip = SyncService::extract_ip_from_url("http://192.168.90.11").unwrap();
        assert_eq!(ip, "192.168.90.11");
    }

    #[test]
    fn test_extract_ip_from_url_with_port() {
        let ip = SyncService::extract_ip_from_url("http://192.168.90.11:80").unwrap();
        assert_eq!(ip, "192.168.90.11");
    }

    #[test]
    fn test_extract_ip_from_url_with_path() {
        let ip = SyncService::extract_ip_from_url("http://192.168.90.11/path").unwrap();
        assert_eq!(ip, "192.168.90.11");
    }

    #[test]
    fn test_extract_ip_from_url_https() {
        let ip = SyncService::extract_ip_from_url("https://192.168.90.11").unwrap();
        assert_eq!(ip, "192.168.90.11");
    }

    #[test]
    fn test_extract_ip_from_url_empty() {
        let result = SyncService::extract_ip_from_url("");
        assert!(result.is_err());
    }
}
