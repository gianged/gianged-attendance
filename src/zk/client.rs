//! ZK TCP client for communicating with ZKTeco devices.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use tracing::{debug, info, warn};

use super::attendance::{AttendanceRecord, parse_attendance};
use super::error::{Result, ZkError};
use super::protocol::{
    CHUNK_SIZE, CMD_CONNECT, CMD_DATA_WRRQ, CMD_EXIT, CMD_FREE_DATA, CMD_GET_FREE_SIZES, CMD_READ_CHUNK, HEADER,
    Response, TABLE_ATTLOG, build_packet,
};

/// TCP client for ZKTeco devices.
///
/// Communicates with devices on port 4370 using the ZK binary protocol.
/// Provides blocking I/O operations; wrap in `spawn_blocking` for async usage.
pub struct ZkTcpClient {
    stream: TcpStream,
    session_id: u16,
    reply_id: u16,
}

impl ZkTcpClient {
    /// Connect to a ZKTeco device.
    ///
    /// # Arguments
    /// * `addr` - Device address in format "host:port" (e.g., "192.168.90.11:4370")
    ///
    /// # Errors
    /// Returns `ZkError::Io` on connection failure.
    pub fn connect(addr: &str) -> Result<Self> {
        info!("Connecting to ZK device at {addr}");

        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let mut client = Self {
            stream,
            session_id: 0,
            reply_id: 0,
        };

        // Send connect command
        let response = client.send_command(CMD_CONNECT, &[])?;
        client.session_id = response.session_id;

        info!("Connected to ZK device, session_id={:#06x}", client.session_id);
        Ok(client)
    }

    /// Disconnect from the device.
    pub fn disconnect(&mut self) -> Result<()> {
        debug!("Disconnecting from ZK device");
        self.send_command(CMD_EXIT, &[])?;
        Ok(())
    }

    /// Get all attendance records from device.
    ///
    /// Reads the complete ATTLOG table from device flash storage.
    pub fn get_attendance(&mut self) -> Result<Vec<AttendanceRecord>> {
        info!("Fetching attendance records from device");

        // Get device info first (required by protocol)
        self.send_command(CMD_GET_FREE_SIZES, &[])?;
        self.send_command(CMD_GET_FREE_SIZES, &[])?;

        // Prepare ATTLOG read
        self.send_command(CMD_DATA_WRRQ, &TABLE_ATTLOG)?;

        // Get total size (first chunk request with size query)
        let size_query = [0x00, 0x00, 0x00, 0x00, 0x04, 0x24, 0x00, 0x00];
        let response = self.send_command(CMD_READ_CHUNK, &size_query)?;

        // Parse total data size from response
        let total_size = if response.data.len() >= 4 {
            u32::from_le_bytes([response.data[0], response.data[1], response.data[2], response.data[3]])
        } else {
            return Err(ZkError::NoData);
        };

        info!("Total attendance data size: {total_size} bytes");

        // Free this initial buffer
        self.send_command(CMD_FREE_DATA, &[])?;

        // Now read actual attendance data
        // Prepare again
        self.send_command(CMD_DATA_WRRQ, &TABLE_ATTLOG)?;

        let mut all_data = Vec::with_capacity(total_size as usize);
        let mut offset: u32 = 0;

        while offset < total_size {
            let remaining = total_size - offset;
            let chunk_size = remaining.min(CHUNK_SIZE);

            let mut chunk_req = [0u8; 8];
            chunk_req[0..4].copy_from_slice(&offset.to_le_bytes());
            chunk_req[4..8].copy_from_slice(&chunk_size.to_le_bytes());

            let response = self.send_command(CMD_READ_CHUNK, &chunk_req)?;
            all_data.extend_from_slice(&response.data);

            debug!(
                "Read chunk: offset={offset}, size={}, received={}",
                chunk_size,
                response.data.len()
            );

            offset += chunk_size;
        }

        // Free buffer
        self.send_command(CMD_FREE_DATA, &[])?;

        info!("Downloaded {} bytes of attendance data", all_data.len());

        // Parse records
        let records = parse_attendance(&all_data);
        info!("Parsed {} attendance records", records.len());

        Ok(records)
    }

    /// Send a command to the device and read response.
    fn send_command(&mut self, cmd: u16, data: &[u8]) -> Result<Response> {
        let packet = build_packet(cmd, self.session_id, self.reply_id, data);
        self.stream.write_all(&packet)?;
        self.reply_id = self.reply_id.wrapping_add(1);

        self.read_response()
    }

    /// Read a response from the device.
    fn read_response(&mut self) -> Result<Response> {
        // Read header (8 bytes)
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header)?;

        // Verify header
        if header[0..4] != HEADER {
            return Err(ZkError::InvalidResponse("Bad header".to_string()));
        }

        let payload_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

        // Read payload
        let mut payload = vec![0u8; payload_size];
        self.stream.read_exact(&mut payload)?;

        // Parse inner packet
        if payload.len() >= 8 {
            Ok(Response {
                cmd: u16::from_le_bytes([payload[0], payload[1]]),
                session_id: u16::from_le_bytes([payload[4], payload[5]]),
                reply_id: u16::from_le_bytes([payload[6], payload[7]]),
                data: payload[8..].to_vec(),
            })
        } else {
            Err(ZkError::InvalidResponse("Payload too small".to_string()))
        }
    }
}

impl Drop for ZkTcpClient {
    fn drop(&mut self) {
        if let Err(e) = self.disconnect() {
            warn!("Failed to disconnect from ZK device: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require a real device, mark as ignored
    #[test]
    #[ignore]
    fn test_real_device_connection() {
        use super::*;

        let mut client = ZkTcpClient::connect("192.168.90.11:4370").expect("Failed to connect to device");

        let records = client.get_attendance().expect("Failed to get attendance");
        println!("Retrieved {} records", records.len());

        assert!(!records.is_empty(), "Expected some attendance records");
    }
}
