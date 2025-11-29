//! ZKTeco HTTP client implementation.

use crate::error::{AppError, Result};
use crate::models::attendance::CreateAttendanceLog;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use reqwest::{Client, cookie::Jar};
use std::sync::Arc;

/// ZKTeco device HTTP client.
///
/// Communicates with ZKTeco fingerprint devices using their CSL HTTP interface.
/// Uses session-based authentication with cookies.
pub struct ZkClient {
    client: Client,
    base_url: String,
    logged_in: bool,
}

impl ZkClient {
    /// Create a new client instance.
    ///
    /// # Arguments
    /// * `base_url` - The device URL (e.g., "http://192.168.90.11")
    pub fn new(base_url: &str) -> Self {
        let jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_provider(jar)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            logged_in: false,
        }
    }

    /// Authenticate with the device.
    ///
    /// Verifies login success by checking the response body for error indicators.
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let url = format!("{base}/csl/check", base = self.base_url);

        let response = self
            .client
            .post(&url)
            .form(&[("username", username), ("userpwd", password)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::DeviceLoginFailed);
        }

        let body = response.text().await?;
        let body_lower = body.to_lowercase();

        // Check for indicators that login failed:
        // - "error" or "invalid" in response suggests authentication failure
        // - Presence of login form fields suggests we're still on login page
        if body_lower.contains("error")
            || body_lower.contains("invalid")
            || (body_lower.contains("username") && body_lower.contains("password"))
        {
            return Err(AppError::DeviceLoginFailed);
        }

        self.logged_in = true;
        Ok(())
    }

    /// Check if currently logged in.
    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }

    /// Download attendance data for a date range.
    ///
    /// # Arguments
    /// * `start_date` - Start of date range
    /// * `end_date` - End of date range
    /// * `user_ids` - List of device user IDs to fetch
    pub async fn download_attendance(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        user_ids: &[i32],
    ) -> Result<Vec<CreateAttendanceLog>> {
        if !self.logged_in {
            return Err(AppError::DeviceLoginFailed);
        }

        let url = format!("{base}/form/Download", base = self.base_url);

        // Build form data with repeated uid parameters
        let mut form_data: Vec<(&str, String)> = vec![
            ("sdate", start_date.format("%Y-%m-%d").to_string()),
            ("edate", end_date.format("%Y-%m-%d").to_string()),
            ("period", "1".to_string()),
        ];

        for uid in user_ids {
            form_data.push(("uid", uid.to_string()));
        }

        let response = self.client.post(&url).form(&form_data).send().await?;
        let body = response.text().await?;

        self.parse_attendance_data(&body)
    }

    /// Parse TSV attendance data from device response.
    ///
    /// Format: `scanner_uid \t [empty] \t timestamp \t verify_type \t status`
    fn parse_attendance_data(&self, data: &str) -> Result<Vec<CreateAttendanceLog>> {
        let mut records = Vec::new();

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Format: scanner_uid \t [empty] \t timestamp \t verify_type \t status
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 5 {
                continue;
            }

            let scanner_uid = match parts[0].trim().parse::<i32>() {
                Ok(uid) => uid,
                Err(_) => continue,
            };

            // parts[1] is empty
            let timestamp_str = parts[2].trim();
            let check_time = match self.parse_local_timestamp(timestamp_str) {
                Ok(dt) => dt,
                Err(_) => continue,
            };

            let verify_type = parts[3].trim().parse::<i32>().unwrap_or(2);
            let status = parts[4].trim().parse::<i32>().unwrap_or(0);

            records.push(CreateAttendanceLog {
                scanner_uid,
                check_time,
                verify_type,
                status,
                source: "device".to_string(),
            });
        }

        Ok(records)
    }

    /// Parse a local timestamp string and convert to UTC.
    fn parse_local_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        let naive_dt = NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| AppError::parse(format!("Invalid timestamp '{timestamp_str}': {e}")))?;

        // Device returns local time, convert to UTC
        let local_dt = Local
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| AppError::parse(format!("Ambiguous local time: {timestamp_str}")))?;

        Ok(local_dt.with_timezone(&Utc))
    }

    /// Test connection to the device.
    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!("{base}/", base = self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attendance_line() {
        let client = ZkClient::new("http://localhost");
        let data = "20\t\t2025-11-25 07:36:58\t2\t0\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].scanner_uid, 20);
        assert_eq!(records[0].verify_type, 2);
        assert_eq!(records[0].status, 0);
        assert_eq!(records[0].source, "device");
    }

    #[test]
    fn test_parse_multiple_lines() {
        let client = ZkClient::new("http://localhost");
        let data = "20\t\t2025-11-25 07:36:58\t2\t0\n65\t\t2025-11-25 07:09:02\t2\t0\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].scanner_uid, 20);
        assert_eq!(records[1].scanner_uid, 65);
    }

    #[test]
    fn test_skip_invalid_lines() {
        let client = ZkClient::new("http://localhost");
        let data = "invalid\nline\n20\t\t2025-11-25 07:36:58\t2\t0\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_skip_empty_lines() {
        let client = ZkClient::new("http://localhost");
        let data = "\n\n20\t\t2025-11-25 07:36:58\t2\t0\n\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_parse_local_timestamp() {
        let client = ZkClient::new("http://localhost");
        let result = client.parse_local_timestamp("2025-11-25 07:36:58");

        assert!(result.is_ok());
        let dt = result.unwrap();
        // Exact UTC time depends on local timezone, but should parse successfully
        assert!(dt.timestamp() > 0);
    }

    #[test]
    fn test_parse_invalid_timestamp() {
        let client = ZkClient::new("http://localhost");
        let result = client.parse_local_timestamp("invalid");

        assert!(result.is_err());
    }
}
