# Phase 11: ZKTeco HTTP Client

## Objective

Implement HTTP client for communicating with ZKTeco fingerprint device.

---

## Tasks

### 11.1 Create Client Module

**`src/client.rs`**

```rust
use crate::error::{AppError, Result};
use crate::models::attendance::CreateAttendanceLog;
use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use reqwest::{cookie::Jar, Client};
use std::sync::Arc;

/// ZKTeco device HTTP client
pub struct ZkClient {
    client: Client,
    base_url: String,
    logged_in: bool,
}

impl ZkClient {
    /// Create a new client instance
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

    /// Authenticate with the device
    pub async fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let url = format!("{}/csl/check", self.base_url);

        let response = self
            .client
            .post(&url)
            .form(&[("username", username), ("userpwd", password)])
            .send()
            .await?;

        if response.status().is_success() {
            self.logged_in = true;
            Ok(())
        } else {
            Err(AppError::DeviceLoginFailed)
        }
    }

    /// Check if logged in
    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }

    /// Download attendance data for date range
    pub async fn download_attendance(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        user_ids: &[i32],
    ) -> Result<Vec<CreateAttendanceLog>> {
        if !self.logged_in {
            return Err(AppError::DeviceLoginFailed);
        }

        let url = format!("{}/form/Download", self.base_url);

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

    /// Parse TSV attendance data
    fn parse_attendance_data(&self, data: &str) -> Result<Vec<CreateAttendanceLog>> {
        let mut records = Vec::new();

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Format: device_uid \t [empty] \t timestamp \t verify_type \t status
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 5 {
                continue;
            }

            let device_uid = match parts[0].trim().parse::<i32>() {
                Ok(uid) => uid,
                Err(_) => continue,
            };

            // parts[1] is empty
            let timestamp_str = parts[2].trim();
            let check_time =
                match NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => Utc.from_utc_datetime(&dt),
                    Err(_) => continue,
                };

            let verify_type = parts[3].trim().parse::<i32>().unwrap_or(2);
            let status = parts[4].trim().parse::<i32>().unwrap_or(0);

            records.push(CreateAttendanceLog {
                device_uid,
                check_time,
                verify_type,
                status,
                source: "device".to_string(),
            });
        }

        Ok(records)
    }

    /// Test connection to device
    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}
```

### 11.2 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attendance_line() {
        let client = ZkClient::new("http://localhost");
        let data = "20\t\t2025-11-25 07:36:58\t2\t0\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].device_uid, 20);
        assert_eq!(records[0].verify_type, 2);
    }

    #[test]
    fn test_parse_multiple_lines() {
        let client = ZkClient::new("http://localhost");
        let data = "20\t\t2025-11-25 07:36:58\t2\t0\n65\t\t2025-11-25 07:09:02\t2\t0\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_skip_invalid_lines() {
        let client = ZkClient::new("http://localhost");
        let data = "invalid\nline\n20\t\t2025-11-25 07:36:58\t2\t0\n";
        let records = client.parse_attendance_data(data).unwrap();

        assert_eq!(records.len(), 1);
    }
}
```

---

## Deliverables

- [x] ZkClient struct
- [x] new() constructor with cookie jar
- [x] login() with form data
- [x] download_attendance() function
- [x] parse_attendance_data() TSV parser
- [x] test_connection() function
- [x] Unit tests for parsing
