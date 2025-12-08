//! ZKTeco TCP binary protocol client.
//!
//! Implements the binary protocol on port 4370 for complete attendance data retrieval.
//! Based on the pyzk library implementation.

use crate::error::{AppError, Result};
use crate::models::attendance::CreateAttendanceLog;
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, trace};

// Command codes
const CMD_CONNECT: u16 = 1000;
const CMD_EXIT: u16 = 1001;
const CMD_DISABLEDEVICE: u16 = 1003;
const CMD_ENABLEDEVICE: u16 = 1004;
const CMD_GET_FREE_SIZES: u16 = 50;
const CMD_ATTLOG_RRQ: u16 = 13;

// Data transfer commands
const CMD_PREPARE_DATA: u16 = 1500;
const CMD_DATA: u16 = 1501;
const CMD_FREE_DATA: u16 = 1502;
const CMD_PREPARE_BUFFER: u16 = 1503;
const CMD_READ_BUFFER: u16 = 1504;

// Response codes
const CMD_ACK_OK: u16 = 2000;

// Protocol constants
const TCP_HEADER: [u8; 4] = [0x50, 0x50, 0x82, 0x7D];
const HEADER_SIZE: usize = 8;
const PAYLOAD_MIN_SIZE: usize = 8; // cmd(2) + checksum(2) + session(2) + reply(2)
const MAX_CHUNK: usize = 0xFFC0; // ~65KB per chunk for TCP

/// Response from device including metadata.
struct TcpResponse {
    /// Response code from device
    code: u16,
    /// Payload data (after 8-byte header)
    data: Vec<u8>,
    /// Expected TCP payload length from header
    tcp_length: usize,
}

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

        debug!("Starting attendance download");

        // Lock device during data transfer
        debug!("Disabling device");
        self.send_command(CMD_DISABLEDEVICE, &[]).await?;

        // Use buffered read for attendance data
        // Note: read_with_buffer handles CMD_FREE_DATA internally for chunked reads
        let raw_data = match self.read_with_buffer(CMD_ATTLOG_RRQ).await {
            Ok(data) => {
                debug!("Downloaded {} bytes of raw data", data.len());
                data
            }
            Err(e) => {
                // Cleanup on error
                debug!("Error during download: {e}, cleaning up");
                let _ = self.send_command(CMD_FREE_DATA, &[]).await;
                let _ = self.send_command(CMD_ENABLEDEVICE, &[]).await;
                return Err(e);
            }
        };

        // Unlock device
        debug!("Re-enabling device");
        let _ = self.send_command(CMD_ENABLEDEVICE, &[]).await;

        // Parse attendance data
        debug!("Parsing attendance data");
        self.parse_attendance_data(&raw_data)
    }

    /// Read data using buffered commands (for large data like attendance logs).
    /// This follows pyzk's read_with_buffer implementation exactly.
    async fn read_with_buffer(&mut self, command: u16) -> Result<Vec<u8>> {
        // Build command string for PREPARE_BUFFER:
        // pyzk uses: pack('<bhii', 1, command, fct, ext)
        // <b = signed byte, <h = signed short (2 bytes), <i = signed int (4 bytes) x2
        let mut cmd_data = Vec::with_capacity(11);
        cmd_data.push(1u8); // Always 1
        cmd_data.extend_from_slice(&(command as i16).to_le_bytes()); // Command as signed short
        cmd_data.extend_from_slice(&0i32.to_le_bytes()); // fct = 0
        cmd_data.extend_from_slice(&0i32.to_le_bytes()); // ext = 0

        debug!("PREPARE_BUFFER: command={command}, data={cmd_data:02X?}");

        // Send PREPARE_BUFFER command and get full response
        let response = self.send_command_full(CMD_PREPARE_BUFFER, &cmd_data).await?;

        debug!(
            "PREPARE_BUFFER response: code={}, data_len={}, tcp_length={}",
            response.code,
            response.data.len(),
            response.tcp_length
        );

        // If response is CMD_DATA, data is included directly or follows
        if response.code == CMD_DATA {
            // Check if we have all the data
            // tcp_length includes the 8-byte header, so actual data = tcp_length - 8
            let expected_data_len = response.tcp_length.saturating_sub(PAYLOAD_MIN_SIZE);
            let have_data_len = response.data.len();

            debug!("CMD_DATA: have {have_data_len} bytes, expected {expected_data_len} bytes");

            if have_data_len < expected_data_len {
                // Need to read more raw data from socket
                let need = expected_data_len - have_data_len;
                debug!("Need {need} more bytes of raw data");
                let more_data = self.receive_raw_data(need).await?;
                let mut result = response.data;
                result.extend_from_slice(&more_data);
                return Ok(result);
            }

            return Ok(response.data);
        }

        // CMD_PREPARE_DATA response - size is at offset 1-5 in data portion
        if response.data.len() < 5 {
            return Err(AppError::TcpProtocolError(format!(
                "PREPARE_BUFFER response data too small: {} bytes",
                response.data.len()
            )));
        }

        let size =
            u32::from_le_bytes([response.data[1], response.data[2], response.data[3], response.data[4]]) as usize;

        debug!("PREPARE_DATA: total size = {size} bytes");

        if size == 0 {
            return Ok(Vec::new());
        }

        // Calculate number of chunks
        let remain = size % MAX_CHUNK;
        let packets = (size - remain) / MAX_CHUNK;

        debug!("Reading {packets} full chunks + {remain} bytes remaining");

        let mut all_data = Vec::with_capacity(size);
        let mut start: u32 = 0;

        // Read full chunks
        for i in 0..packets {
            debug!("Reading chunk {i}/{packets} at offset {start}", i = i + 1);
            let chunk = self.read_chunk(start, MAX_CHUNK as u32).await?;
            debug!("Chunk {} returned {} bytes", i + 1, chunk.len());
            all_data.extend_from_slice(&chunk);
            start += MAX_CHUNK as u32;
        }

        // Read remaining data
        if remain > 0 {
            debug!("Reading final {remain} bytes at offset {start}");
            let chunk = self.read_chunk(start, remain as u32).await?;
            debug!("Final chunk returned {} bytes", chunk.len());
            all_data.extend_from_slice(&chunk);
        }

        // Call free_data to clean up device buffer
        debug!("Freeing device buffer");
        let _ = self.send_command(CMD_FREE_DATA, &[]).await;

        Ok(all_data)
    }

    /// Read a chunk from the device buffer.
    /// Follows pyzk's __read_chunk implementation.
    async fn read_chunk(&mut self, start: u32, size: u32) -> Result<Vec<u8>> {
        // pyzk uses: pack('<ii', start, size) - signed ints
        let mut cmd_data = Vec::with_capacity(8);
        cmd_data.extend_from_slice(&(start as i32).to_le_bytes());
        cmd_data.extend_from_slice(&(size as i32).to_le_bytes());

        // Retry up to 3 times like pyzk
        for retry in 0..3 {
            if retry > 0 {
                debug!("Retry {retry}/3 for chunk at {start}");
            }

            // Send READ_BUFFER command
            let response = self.send_command_full(CMD_READ_BUFFER, &cmd_data).await?;

            debug!(
                "READ_BUFFER response: code={}, data_len={}, tcp_length={}",
                response.code,
                response.data.len(),
                response.tcp_length
            );

            // Now receive the actual chunk data
            if let Some(data) = self.receive_chunk(&response).await? {
                return Ok(data);
            }
        }

        Err(AppError::TcpProtocolError(format!(
            "Failed to read chunk at offset {start} after 3 retries"
        )))
    }

    /// Receive chunk data after a READ_BUFFER command.
    /// Follows pyzk's __recieve_chunk implementation.
    async fn receive_chunk(&mut self, response: &TcpResponse) -> Result<Option<Vec<u8>>> {
        if response.code == CMD_DATA {
            // Data follows directly - check if we need more
            let expected_data_len = response.tcp_length.saturating_sub(PAYLOAD_MIN_SIZE);
            let have_data_len = response.data.len();

            trace!("receive_chunk CMD_DATA: have={have_data_len}, expected={expected_data_len}");

            if have_data_len < expected_data_len {
                let need = expected_data_len - have_data_len;
                trace!("Need {need} more bytes");
                let more_data = self.receive_raw_data(need).await?;
                let mut result = response.data.clone();
                result.extend_from_slice(&more_data);
                return Ok(Some(result));
            }

            return Ok(Some(response.data.clone()));
        } else if response.code == CMD_PREPARE_DATA {
            // Size is in response, data follows in TCP stream
            let size = self.get_data_size_from_response(response)?;
            trace!("receive_chunk CMD_PREPARE_DATA: size={size}");

            // Read data from TCP stream
            let data = self.receive_tcp_data(response, size).await?;
            return Ok(Some(data));
        }

        trace!("receive_chunk: unexpected response code {}", response.code);
        Ok(None)
    }

    /// Extract data size from PREPARE_DATA response.
    fn get_data_size_from_response(&self, response: &TcpResponse) -> Result<usize> {
        // Size is stored at different offsets depending on response
        if response.data.len() >= 4 {
            // Try offset 0 first (direct size)
            let size =
                u32::from_le_bytes([response.data[0], response.data[1], response.data[2], response.data[3]]) as usize;
            return Ok(size);
        }

        Err(AppError::TcpProtocolError(
            "Cannot extract size from response".to_string(),
        ))
    }

    /// Receive TCP data after PREPARE_DATA response.
    async fn receive_tcp_data(&mut self, response: &TcpResponse, size: usize) -> Result<Vec<u8>> {
        let mut data = Vec::new();

        // First append any data already in the response (after 8 bytes)
        if response.data.len() > 8 {
            data.extend_from_slice(&response.data[8..]);
        }

        // Read remaining data from socket
        if data.len() < size {
            let need = size - data.len();
            let more = self.receive_raw_data(need).await?;
            data.extend_from_slice(&more);
        }

        // Now read the ACK packet
        let ack_response = self.read_response_full().await?;
        if ack_response.code != CMD_ACK_OK {
            debug!("Expected ACK_OK but got {} after data receive", ack_response.code);
        }

        Ok(data)
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

    /// Send a command and receive response with metadata.
    async fn send_command_full(&mut self, command: u16, data: &[u8]) -> Result<TcpResponse> {
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

        // Read response with metadata
        self.read_response_full().await
    }

    /// Send a command and receive response (simple version).
    async fn send_command(&mut self, command: u16, data: &[u8]) -> Result<Vec<u8>> {
        let response = self.send_command_full(command, data).await?;
        // Return full payload including header for backward compatibility
        let mut result = Vec::with_capacity(PAYLOAD_MIN_SIZE + response.data.len());
        result.extend_from_slice(&response.code.to_le_bytes());
        result.extend_from_slice(&[0, 0]); // checksum placeholder
        result.extend_from_slice(&self.session_id.to_le_bytes());
        result.extend_from_slice(&(self.reply_id.wrapping_sub(1)).to_le_bytes());
        result.extend_from_slice(&response.data);
        Ok(result)
    }

    /// Read a response packet with full metadata.
    async fn read_response_full(&mut self) -> Result<TcpResponse> {
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

        // Extract payload length (this is tcp_length)
        let tcp_length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
        trace!("TCP header: payload length = {tcp_length}");

        // Safety limit
        if tcp_length > 1_000_000 {
            return Err(AppError::TcpProtocolError(format!(
                "Payload too large: {tcp_length} bytes"
            )));
        }

        // Read payload
        let mut payload = vec![0u8; tcp_length];
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

        // Extract response code
        let code = u16::from_le_bytes([payload[0], payload[1]]);
        trace!("Response code: {code}");

        // Data is everything after the 8-byte header
        let data = if payload.len() > PAYLOAD_MIN_SIZE {
            payload[PAYLOAD_MIN_SIZE..].to_vec()
        } else {
            Vec::new()
        };

        Ok(TcpResponse { code, data, tcp_length })
    }

    /// Receive raw data from socket (for streaming data after CMD_DATA).
    async fn receive_raw_data(&mut self, size: usize) -> Result<Vec<u8>> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| AppError::TcpConnectionFailed("Not connected".to_string()))?;

        debug!("Receiving {size} bytes of raw data");
        let mut data = vec![0u8; size];
        let mut received = 0;

        while received < size {
            let chunk_size = (size - received).min(65536);
            match timeout(
                self.timeout_duration,
                stream.read(&mut data[received..received + chunk_size]),
            )
            .await
            {
                Ok(Ok(0)) => {
                    // Connection closed
                    break;
                }
                Ok(Ok(n)) => {
                    trace!("Received {n} bytes, total {}/{size}", received + n);
                    received += n;
                }
                Ok(Err(e)) => {
                    return Err(AppError::TcpConnectionFailed(format!("Raw read failed: {e}")));
                }
                Err(_) => {
                    return Err(AppError::DeviceTimeout("Raw data timeout".to_string()));
                }
            }
        }

        data.truncate(received);
        Ok(data)
    }

    /// Parse attendance data (auto-detect format).
    fn parse_attendance_data(&self, data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        // The first 4 bytes might be total size, skip if present
        let data = if data.len() > 4 {
            let potential_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
            if potential_size == data.len() - 4 {
                // First 4 bytes are size header, skip them
                &data[4..]
            } else {
                data
            }
        } else {
            data
        };

        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Detect format: text format has tabs/newlines, binary has structured records
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
