//! ZKTeco TCP binary protocol client.
//!
//! Implements the binary protocol on port 4370 for complete attendance data retrieval.

use crate::error::{AppError, Result};
use crate::models::attendance::CreateAttendanceLog;
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

// Command codes
const CMD_CONNECT: u16 = 1000;
const CMD_EXIT: u16 = 1001;
const CMD_DISABLEDEVICE: u16 = 1003;
const CMD_ENABLEDEVICE: u16 = 1004;
const CMD_GET_FREE_SIZES: u16 = 50;
const CMD_ATTLOG_RRQ: u16 = 13;
const CMD_DATA_RDY: u16 = 1500;
const CMD_FREE_DATA: u16 = 1502;

// Response codes
const CMD_ACK_OK: u16 = 2000;
const CMD_PREPARE_DATA: u16 = 1500;

// Protocol constants
const TCP_HEADER: [u8; 4] = [0x50, 0x50, 0x82, 0x7D];
const HEADER_SIZE: usize = 8;
const PAYLOAD_MIN_SIZE: usize = 8; // cmd(2) + checksum(2) + session(2) + reply(2)

/// ZKTeco TCP client for binary protocol communication.
pub struct ZkTcpClient {
    stream: Option<TcpStream>,
    session_id: u16,
    reply_id: u16,
    ip: String,
    port: u16,
    timeout_duration: Duration,
}

impl ZkTcpClient {
    /// Create a new TCP client.
    pub fn new(ip: &str, port: u16, timeout_secs: u64) -> Self {
        Self {
            stream: None,
            session_id: 0,
            reply_id: 0,
            ip: ip.to_string(),
            port,
            timeout_duration: Duration::from_secs(timeout_secs),
        }
    }

    /// Connect to the device and establish a session.
    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.ip, self.port);

        let stream = timeout(self.timeout_duration, TcpStream::connect(&addr))
            .await
            .map_err(|_| AppError::TcpConnectionFailed(format!("Connection timeout to {addr}")))?
            .map_err(|e| AppError::TcpConnectionFailed(format!("Failed to connect to {addr}: {e}")))?;

        self.stream = Some(stream);

        // Send CMD_CONNECT
        let response = self.send_command(CMD_CONNECT, &[]).await?;

        // Extract session ID from response (bytes 4-6 in payload)
        if response.len() >= 6 {
            self.session_id = u16::from_le_bytes([response[4], response[5]]);
        } else {
            return Err(AppError::TcpProtocolError(
                "Invalid connect response: too short".to_string(),
            ));
        }

        Ok(())
    }

    /// Disconnect from the device.
    pub async fn disconnect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            // Send exit command, ignore errors
            let _ = self.send_command(CMD_EXIT, &[]).await;
            self.stream = None;
            self.session_id = 0;
            self.reply_id = 0;
        }
        Ok(())
    }

    /// Check if connected to the device.
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Get the number of attendance records on the device.
    pub async fn get_attendance_count(&mut self) -> Result<u32> {
        if !self.is_connected() {
            return Err(AppError::TcpConnectionFailed("Not connected".to_string()));
        }

        let response = self.send_command(CMD_GET_FREE_SIZES, &[]).await?;

        // Attendance count is at offset 40 in the response data (after 8-byte payload header)
        // The free sizes response has: users, fingerprints, passwords, cards, attendance logs, etc.
        // Attendance log count is typically at offset 40 (varies by firmware)
        if response.len() >= 44 {
            let count = u32::from_le_bytes([response[40], response[41], response[42], response[43]]);
            Ok(count)
        } else if response.len() >= 12 {
            // Fallback: try offset 8
            let count = u32::from_le_bytes([response[8], response[9], response[10], response[11]]);
            Ok(count)
        } else {
            Err(AppError::TcpProtocolError(format!(
                "Free sizes response too small: {} bytes",
                response.len()
            )))
        }
    }

    /// Download all attendance records from the device.
    pub async fn download_attendance(&mut self) -> Result<Vec<CreateAttendanceLog>> {
        if !self.is_connected() {
            return Err(AppError::TcpConnectionFailed("Not connected".to_string()));
        }

        // Lock device during data transfer
        self.send_command(CMD_DISABLEDEVICE, &[]).await?;

        // Request attendance data
        let response = match self.send_command(CMD_ATTLOG_RRQ, &[]).await {
            Ok(r) => r,
            Err(e) => {
                // Unlock device on error
                let _ = self.send_command(CMD_ENABLEDEVICE, &[]).await;
                return Err(e);
            }
        };

        // Check response code
        let response_code = u16::from_le_bytes([response[0], response[1]]);
        if response_code != CMD_PREPARE_DATA && response_code != CMD_ACK_OK {
            let _ = self.send_command(CMD_ENABLEDEVICE, &[]).await;
            return Err(AppError::TcpProtocolError(format!(
                "Expected PREPARE_DATA response, got {response_code}"
            )));
        }

        // Extract data size from response (offset 8, 4 bytes LE)
        let data_size = if response.len() >= 12 {
            u32::from_le_bytes([response[8], response[9], response[10], response[11]]) as usize
        } else {
            0
        };

        // Read data chunks
        let raw_data = match self.read_data_chunks(data_size).await {
            Ok(data) => data,
            Err(e) => {
                let _ = self.send_command(CMD_FREE_DATA, &[]).await;
                let _ = self.send_command(CMD_ENABLEDEVICE, &[]).await;
                return Err(e);
            }
        };

        // Free device buffer
        let _ = self.send_command(CMD_FREE_DATA, &[]).await;

        // Unlock device
        let _ = self.send_command(CMD_ENABLEDEVICE, &[]).await;

        // Parse attendance data
        self.parse_attendance_data(&raw_data)
    }

    /// Calculate ZKTeco checksum.
    fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;
        for chunk in data.chunks(2) {
            if chunk.len() == 2 {
                sum += u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
            } else {
                sum += chunk[0] as u32;
            }
        }
        // Fold to 16 bits
        while sum > 0xFFFF {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !sum as u16
    }

    /// Build a protocol packet.
    fn build_packet(&mut self, command: u16, data: &[u8]) -> Vec<u8> {
        let payload_len = PAYLOAD_MIN_SIZE + data.len();
        let total_len = HEADER_SIZE + payload_len;

        let mut packet = Vec::with_capacity(total_len);

        // TCP header (magic bytes)
        packet.extend_from_slice(&TCP_HEADER);

        // Payload length (4 bytes LE)
        packet.extend_from_slice(&(payload_len as u32).to_le_bytes());

        // Command (2 bytes LE)
        packet.extend_from_slice(&command.to_le_bytes());

        // Checksum placeholder (2 bytes)
        packet.extend_from_slice(&[0, 0]);

        // Session ID (2 bytes LE)
        packet.extend_from_slice(&self.session_id.to_le_bytes());

        // Reply ID (2 bytes LE)
        packet.extend_from_slice(&self.reply_id.to_le_bytes());

        // Data
        packet.extend_from_slice(data);

        // Calculate and insert checksum (bytes 10-11, covering payload starting at byte 8)
        let checksum = Self::calculate_checksum(&packet[8..]);
        packet[10..12].copy_from_slice(&checksum.to_le_bytes());

        // Increment reply ID for next packet
        self.reply_id = self.reply_id.wrapping_add(1);

        packet
    }

    /// Send a command and receive response.
    async fn send_command(&mut self, command: u16, data: &[u8]) -> Result<Vec<u8>> {
        // Build packet first (needs &mut self for reply_id)
        let packet = self.build_packet(command, data);
        let timeout_duration = self.timeout_duration;

        // Now borrow stream
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| AppError::TcpConnectionFailed("Not connected".to_string()))?;

        // Write packet
        timeout(timeout_duration, stream.write_all(&packet))
            .await
            .map_err(|_| AppError::DeviceTimeout("Write timeout".to_string()))?
            .map_err(|e| AppError::TcpConnectionFailed(format!("Write failed: {e}")))?;

        // Read response (stream borrow ends here)
        self.read_response().await
    }

    /// Read a response packet.
    async fn read_response(&mut self) -> Result<Vec<u8>> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| AppError::TcpConnectionFailed("Not connected".to_string()))?;

        // Read TCP header (8 bytes: magic + length)
        let mut header = [0u8; HEADER_SIZE];
        timeout(self.timeout_duration, stream.read_exact(&mut header))
            .await
            .map_err(|_| AppError::DeviceTimeout("Read timeout".to_string()))?
            .map_err(|e| AppError::TcpConnectionFailed(format!("Read failed: {e}")))?;

        // Verify magic bytes
        if header[0..4] != TCP_HEADER {
            return Err(AppError::TcpProtocolError(format!(
                "Invalid TCP header: {:02X?}",
                &header[0..4]
            )));
        }

        // Extract payload length
        let payload_len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

        // Safety limit
        if payload_len > 1_000_000 {
            return Err(AppError::TcpProtocolError(format!(
                "Payload too large: {payload_len} bytes"
            )));
        }

        // Read payload
        let mut payload = vec![0u8; payload_len];
        timeout(self.timeout_duration, stream.read_exact(&mut payload))
            .await
            .map_err(|_| AppError::DeviceTimeout("Payload read timeout".to_string()))?
            .map_err(|e| AppError::TcpConnectionFailed(format!("Payload read failed: {e}")))?;

        // Verify minimum payload size
        if payload.len() < PAYLOAD_MIN_SIZE {
            return Err(AppError::TcpProtocolError(format!(
                "Payload too small: {} bytes",
                payload.len()
            )));
        }

        Ok(payload)
    }

    /// Read data chunks from device.
    async fn read_data_chunks(&mut self, expected_size: usize) -> Result<Vec<u8>> {
        let mut all_data = Vec::with_capacity(expected_size);
        const MAX_CHUNKS: usize = 200;
        const MAX_DATA_SIZE: usize = 10_000_000; // 10MB safety limit

        for _ in 0..MAX_CHUNKS {
            let response = self.send_command(CMD_DATA_RDY, &[]).await?;

            // Data starts after the 8-byte payload header
            if response.len() > PAYLOAD_MIN_SIZE {
                let chunk = &response[PAYLOAD_MIN_SIZE..];
                all_data.extend_from_slice(chunk);

                // Check if this is the last chunk (small response or expected size reached)
                if chunk.len() < 1024 || all_data.len() >= expected_size {
                    break;
                }
            } else {
                // Empty response, done
                break;
            }

            // Safety check
            if all_data.len() > MAX_DATA_SIZE {
                return Err(AppError::TcpProtocolError(format!(
                    "Data too large: {} bytes",
                    all_data.len()
                )));
            }
        }

        Ok(all_data)
    }

    /// Parse attendance data (auto-detect format).
    fn parse_attendance_data(&self, data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Detect format: text format starts with ASCII digits or has tabs/newlines
        // Binary format has structured 40-byte records with null padding
        let is_text = data
            .iter()
            .take(100)
            .filter(|&&b| b == b'\t' || b == b'\n' || b == b'\r')
            .count()
            > 2;

        if is_text {
            self.parse_text_format(data)
        } else {
            self.parse_binary_format(data)
        }
    }

    /// Parse text (TSV) format attendance data.
    fn parse_text_format(&self, data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
        let text = String::from_utf8_lossy(data);
        let mut records = Vec::new();

        for line in text.lines() {
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

    /// Parse binary format attendance data (40 bytes per record).
    fn parse_binary_format(&self, data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
        const RECORD_SIZE: usize = 40;
        let mut records = Vec::new();

        for chunk in data.chunks_exact(RECORD_SIZE) {
            // Extract user ID (offset 0-9, null-terminated string)
            let user_id_end = chunk[0..9].iter().position(|&b| b == 0).unwrap_or(9);
            let user_id_str = match std::str::from_utf8(&chunk[0..user_id_end]) {
                Ok(s) => s.trim(),
                Err(_) => continue,
            };

            let scanner_uid = match user_id_str.parse::<i32>() {
                Ok(uid) => uid,
                Err(_) => continue,
            };

            // Extract timestamp (offset 24-28, 4 bytes LE, seconds since 2000-01-01)
            let timestamp_raw = u32::from_le_bytes([chunk[24], chunk[25], chunk[26], chunk[27]]);

            // Convert from seconds since 2000-01-01 to DateTime
            let base = match NaiveDate::from_ymd_opt(2000, 1, 1).and_then(|d| d.and_hms_opt(0, 0, 0)) {
                Some(dt) => dt,
                None => continue,
            };

            let naive_dt = base + chrono::Duration::seconds(i64::from(timestamp_raw));

            // Convert local time to UTC
            let check_time = match Local.from_local_datetime(&naive_dt).single() {
                Some(local_dt) => local_dt.with_timezone(&Utc),
                None => continue, // Skip ambiguous times
            };

            // Extract verify type (offset 28)
            let verify_type = i32::from(chunk[28]);

            // Extract status (offset 29)
            let status = i32::from(chunk[29]);

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

    /// Parse a local timestamp string to UTC.
    fn parse_local_timestamp(&self, timestamp_str: &str) -> Result<DateTime<Utc>> {
        let naive_dt = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| AppError::parse(format!("Invalid timestamp '{timestamp_str}': {e}")))?;

        let local_dt = Local
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| AppError::parse(format!("Ambiguous local time: {timestamp_str}")))?;

        Ok(local_dt.with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_checksum() {
        // Test with known data (CMD_CONNECT packet payload with zeros)
        let data = [0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let checksum = ZkTcpClient::calculate_checksum(&data);

        // Checksum should be non-zero for this data
        assert!(checksum > 0);
    }

    #[test]
    fn test_checksum_empty_data() {
        let data: [u8; 0] = [];
        let checksum = ZkTcpClient::calculate_checksum(&data);
        assert_eq!(checksum, 0xFFFF); // Complement of 0
    }

    #[test]
    fn test_build_packet_structure() {
        let mut client = ZkTcpClient::new("127.0.0.1", 4370, 30);
        let packet = client.build_packet(CMD_CONNECT, &[]);

        // Verify TCP header
        assert_eq!(&packet[0..4], &TCP_HEADER);

        // Verify payload length
        let payload_len = u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]);
        assert_eq!(payload_len as usize, PAYLOAD_MIN_SIZE);

        // Verify command
        let command = u16::from_le_bytes([packet[8], packet[9]]);
        assert_eq!(command, CMD_CONNECT);

        // Verify total packet size
        assert_eq!(packet.len(), HEADER_SIZE + PAYLOAD_MIN_SIZE);
    }

    #[test]
    fn test_build_packet_with_data() {
        let mut client = ZkTcpClient::new("127.0.0.1", 4370, 30);
        let extra_data = [0x01, 0x02, 0x03, 0x04];
        let packet = client.build_packet(CMD_CONNECT, &extra_data);

        // Verify payload length includes extra data
        let payload_len = u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]);
        assert_eq!(payload_len as usize, PAYLOAD_MIN_SIZE + extra_data.len());

        // Verify extra data is at the end
        assert_eq!(&packet[16..20], &extra_data);
    }

    #[test]
    fn test_parse_text_format() {
        let client = ZkTcpClient::new("127.0.0.1", 4370, 30);
        let data = b"20\t\t2025-12-02 07:36:58\t2\t0\n65\t\t2025-12-02 08:15:23\t2\t0\n";

        let records = client.parse_text_format(data).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].scanner_uid, 20);
        assert_eq!(records[0].verify_type, 2);
        assert_eq!(records[0].status, 0);
        assert_eq!(records[1].scanner_uid, 65);
    }

    #[test]
    fn test_parse_text_format_skip_invalid() {
        let client = ZkTcpClient::new("127.0.0.1", 4370, 30);
        let data = b"invalid\t\t2025-12-02 07:36:58\t2\t0\n20\t\t2025-12-02 08:15:23\t2\t0\n";

        let records = client.parse_text_format(data).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].scanner_uid, 20);
    }

    #[test]
    fn test_parse_binary_format() {
        let client = ZkTcpClient::new("127.0.0.1", 4370, 30);

        // Create a test record (40 bytes)
        let mut record = vec![0u8; 40];

        // User ID "20" at offset 0
        record[0] = b'2';
        record[1] = b'0';

        // Timestamp: 1 day after 2000-01-01 = 86400 seconds
        let timestamp: u32 = 86400;
        record[24..28].copy_from_slice(&timestamp.to_le_bytes());

        // Verify type: 2
        record[28] = 2;

        // Status: 0
        record[29] = 0;

        let records = client.parse_binary_format(&record).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].scanner_uid, 20);
        assert_eq!(records[0].verify_type, 2);
        assert_eq!(records[0].status, 0);
    }

    #[test]
    fn test_parse_binary_format_multiple_records() {
        let client = ZkTcpClient::new("127.0.0.1", 4370, 30);

        // Create two test records (80 bytes total)
        let mut data = vec![0u8; 80];

        // First record: UID 20
        data[0] = b'2';
        data[1] = b'0';
        let ts1: u32 = 86400;
        data[24..28].copy_from_slice(&ts1.to_le_bytes());
        data[28] = 2;

        // Second record: UID 65
        data[40] = b'6';
        data[41] = b'5';
        let ts2: u32 = 172800; // 2 days
        data[64..68].copy_from_slice(&ts2.to_le_bytes());
        data[68] = 2;

        let records = client.parse_binary_format(&data).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].scanner_uid, 20);
        assert_eq!(records[1].scanner_uid, 65);
    }

    #[test]
    fn test_reply_id_increments() {
        let mut client = ZkTcpClient::new("127.0.0.1", 4370, 30);
        assert_eq!(client.reply_id, 0);

        let _ = client.build_packet(CMD_CONNECT, &[]);
        assert_eq!(client.reply_id, 1);

        let _ = client.build_packet(CMD_EXIT, &[]);
        assert_eq!(client.reply_id, 2);
    }
}
