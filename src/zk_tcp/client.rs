//! ZkTcpClient struct and public API.

use super::io::{read_response, write_packet};
use super::parser::parse_attendance_data;
use super::protocol::build_packet;
use super::transfer::read_with_buffer;
use super::types::{
    CMD_ATTLOG_RRQ, CMD_CONNECT, CMD_DISABLEDEVICE, CMD_ENABLEDEVICE, CMD_EXIT, CMD_FREE_DATA, CMD_GET_FREE_SIZES,
    PAYLOAD_MIN_SIZE,
};
use crate::error::{AppError, Result};
use crate::models::attendance::CreateAttendanceLog;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info};

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

        println!("[ZK] TCP connecting to {} (timeout={:?})", addr, self.timeout_duration);
        info!("TCP connecting to {} (timeout={:?})", addr, self.timeout_duration);

        let stream = timeout(self.timeout_duration, TcpStream::connect(&addr))
            .await
            .map_err(|_| {
                error!("Connection timeout to {addr}");
                AppError::TcpConnectionFailed(format!("Connection timeout to {addr}"))
            })?
            .map_err(|e| {
                error!("Failed to connect to {addr}: {e}");
                AppError::TcpConnectionFailed(format!("Failed to connect to {addr}: {e}"))
            })?;

        self.stream = Some(stream);
        println!("[ZK] TCP connected OK, sending CMD_CONNECT...");

        // Send CMD_CONNECT
        let response = self.send_command(CMD_CONNECT, &[]).await?;
        println!("[ZK] CMD_CONNECT response: {} bytes", response.len());

        // Extract session ID from response (bytes 4-6 in payload)
        if response.len() >= 6 {
            self.session_id = u16::from_le_bytes([response[4], response[5]]);
            info!("Connected to device, session_id={}", self.session_id);
        } else {
            error!("Invalid connect response: too short ({} bytes)", response.len());
            return Err(AppError::TcpProtocolError(
                "Invalid connect response: too short".to_string(),
            ));
        }

        Ok(())
    }

    /// Disconnect from the device.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut stream) = self.stream {
            info!("Disconnecting from device");
            // Send CMD_EXIT but don't wait for response (device may not respond)
            let packet = build_packet(CMD_EXIT, &[], self.session_id, &mut self.reply_id);
            // Fire and forget - ignore write errors
            let _ = write_packet(stream, &packet, Duration::from_secs(2)).await;
        }
        self.stream = None;
        self.session_id = 0;
        self.reply_id = 0;
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
            error!("Free sizes response too small: {} bytes", response.len());
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
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| AppError::TcpConnectionFailed("Not connected".to_string()))?;

        let raw_data = match read_with_buffer(
            stream,
            CMD_ATTLOG_RRQ,
            self.session_id,
            &mut self.reply_id,
            self.timeout_duration,
        )
        .await
        {
            Ok(data) => {
                debug!("Downloaded {} bytes of raw data", data.len());
                data
            }
            Err(e) => {
                // Cleanup on error
                error!("Error during download: {e}, cleaning up");
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
        parse_attendance_data(&raw_data)
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

    /// Send a command and receive response with metadata.
    async fn send_command_full(&mut self, command: u16, data: &[u8]) -> Result<super::types::TcpResponse> {
        // Build packet first (needs reply_id)
        let packet = build_packet(command, data, self.session_id, &mut self.reply_id);
        let timeout_duration = self.timeout_duration;

        // Now borrow stream
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| AppError::TcpConnectionFailed("Not connected".to_string()))?;

        // Write packet
        write_packet(stream, &packet, timeout_duration).await?;

        // Read response with metadata
        read_response(stream, timeout_duration).await
    }

    /// Diagnose connection issues. Returns detailed status info.
    pub async fn diagnose_connection(&mut self) -> ConnectionDiagnosis {
        let addr = format!("{}:{}", self.ip, self.port);
        let start = std::time::Instant::now();

        // Step 1: Try TCP connect
        let tcp_result = timeout(self.timeout_duration, TcpStream::connect(&addr)).await;

        let tcp_connect_ms = start.elapsed().as_millis() as u64;

        match tcp_result {
            Err(_) => {
                return ConnectionDiagnosis {
                    tcp_reachable: false,
                    tcp_connect_ms,
                    tcp_error: Some("TCP connection timeout".to_string()),
                    protocol_ok: false,
                    protocol_error: None,
                    session_id: None,
                    device_response_code: None,
                };
            }
            Ok(Err(e)) => {
                return ConnectionDiagnosis {
                    tcp_reachable: false,
                    tcp_connect_ms,
                    tcp_error: Some(format!("TCP error: {e}")),
                    protocol_ok: false,
                    protocol_error: None,
                    session_id: None,
                    device_response_code: None,
                };
            }
            Ok(Ok(stream)) => {
                self.stream = Some(stream);
            }
        }

        // Step 2: Try CMD_CONNECT
        let protocol_start = std::time::Instant::now();
        let cmd_result = self.send_command_full(CMD_CONNECT, &[]).await;
        let _protocol_ms = protocol_start.elapsed().as_millis() as u64;

        match cmd_result {
            Err(e) => {
                self.stream = None;
                ConnectionDiagnosis {
                    tcp_reachable: true,
                    tcp_connect_ms,
                    tcp_error: None,
                    protocol_ok: false,
                    protocol_error: Some(format!("{e}")),
                    session_id: None,
                    device_response_code: None,
                }
            }
            Ok(response) => {
                let session_id = if response.data.len() >= 2 {
                    Some(u16::from_le_bytes([response.data[0], response.data[1]]))
                } else {
                    None
                };

                // Clean disconnect
                let _ = self.send_command(CMD_EXIT, &[]).await;
                self.stream = None;
                self.session_id = 0;
                self.reply_id = 0;

                ConnectionDiagnosis {
                    tcp_reachable: true,
                    tcp_connect_ms,
                    tcp_error: None,
                    protocol_ok: true,
                    protocol_error: None,
                    session_id,
                    device_response_code: Some(response.code),
                }
            }
        }
    }

    /// Get reply_id for testing purposes.
    #[cfg(test)]
    pub(crate) fn reply_id(&self) -> u16 {
        self.reply_id
    }

    /// Build packet for testing purposes.
    #[cfg(test)]
    pub(crate) fn build_packet_for_test(&mut self, command: u16, data: &[u8]) -> Vec<u8> {
        build_packet(command, data, self.session_id, &mut self.reply_id)
    }
}

/// Diagnostic information about connection attempt.
#[derive(Debug, Clone)]
pub struct ConnectionDiagnosis {
    /// Whether TCP port was reachable
    pub tcp_reachable: bool,
    /// Time to establish TCP connection (ms)
    pub tcp_connect_ms: u64,
    /// TCP-level error if any
    pub tcp_error: Option<String>,
    /// Whether ZKTeco protocol handshake succeeded
    pub protocol_ok: bool,
    /// Protocol-level error if any
    pub protocol_error: Option<String>,
    /// Session ID from device (if connected)
    pub session_id: Option<u16>,
    /// Response code from CMD_CONNECT
    pub device_response_code: Option<u16>,
}

impl std::fmt::Display for ConnectionDiagnosis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Connection Diagnosis ===")?;
        writeln!(f, "TCP Reachable: {}", self.tcp_reachable)?;
        writeln!(f, "TCP Connect Time: {}ms", self.tcp_connect_ms)?;

        if let Some(ref err) = self.tcp_error {
            writeln!(f, "TCP Error: {err}")?;
        }

        if self.tcp_reachable {
            writeln!(f, "Protocol OK: {}", self.protocol_ok)?;
            if let Some(ref err) = self.protocol_error {
                writeln!(f, "Protocol Error: {err}")?;
            }
            if let Some(code) = self.device_response_code {
                writeln!(f, "Device Response Code: {code}")?;
            }
            if let Some(sid) = self.session_id {
                writeln!(f, "Session ID: {sid}")?;
            }
        }

        Ok(())
    }
}
