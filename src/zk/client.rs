//! ZK TCP client for communicating with ZKTeco devices.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use tracing::{debug, info, warn};

use super::attendance::{AttendanceRecord, parse_attendance};
use super::error::{Result, ZkError};
use super::protocol::{
    CHUNK_SIZE, CMD_ACK_DATA, CMD_ACK_OK, CMD_CLEAR_ATTLOG, CMD_CONNECT, CMD_DATA, CMD_DATA_WRRQ, CMD_EXIT,
    CMD_FREE_DATA, CMD_GET_FREE_SIZES, CMD_READ_CHUNK, HEADER, Response, TABLE_ATTLOG, build_packet,
};

/// Device storage capacity information.
#[derive(Debug, Clone)]
pub struct DeviceCapacity {
    /// Current attendance record count.
    pub records: u32,
    /// Maximum record capacity.
    pub records_cap: u32,
    /// Available record slots.
    pub records_av: u32,
}

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

    /// Get device storage capacity information.
    pub fn get_capacity(&mut self) -> Result<DeviceCapacity> {
        debug!("Getting device capacity");

        let response = self.send_command(CMD_GET_FREE_SIZES, &[])?;

        // Response contains 20 u32 values (80 bytes)
        if response.data.len() < 80 {
            return Err(ZkError::InvalidResponse(format!(
                "Expected 80 bytes for capacity info, got {}",
                response.data.len()
            )));
        }

        let get_u32 = |idx: usize| -> u32 {
            let offset = idx * 4;
            u32::from_le_bytes([
                response.data[offset],
                response.data[offset + 1],
                response.data[offset + 2],
                response.data[offset + 3],
            ])
        };

        let capacity = DeviceCapacity {
            records: get_u32(8),
            records_cap: get_u32(16),
            records_av: get_u32(19),
        };

        info!(
            "Device capacity: {} / {} records ({} available)",
            capacity.records, capacity.records_cap, capacity.records_av
        );

        Ok(capacity)
    }

    /// Clear all attendance records from device.
    pub fn clear_attendance(&mut self) -> Result<()> {
        info!("Clearing attendance records from device");

        let response = self.send_command(CMD_CLEAR_ATTLOG, &[])?;

        if response.cmd != CMD_ACK_OK {
            return Err(ZkError::InvalidResponse(format!(
                "Expected CMD_ACK_OK ({CMD_ACK_OK}) after clear, got {}",
                response.cmd
            )));
        }

        info!("Attendance records cleared successfully");
        Ok(())
    }

    /// Get all attendance records from device.
    ///
    /// Reads the complete ATTLOG table from device flash storage.
    /// First gets total size from DATA_WRRQ response, then reads chunks
    /// with exact sizes to avoid requesting beyond available data.
    pub fn get_attendance(&mut self) -> Result<Vec<AttendanceRecord>> {
        info!("Fetching attendance records from device");

        // Get device info first (required by protocol)
        self.send_command(CMD_GET_FREE_SIZES, &[])?;
        self.send_command(CMD_GET_FREE_SIZES, &[])?;

        // Send DATA_WRRQ - device responds with ACK_OK containing total size
        let wrrq_response = self.send_command(CMD_DATA_WRRQ, &TABLE_ATTLOG)?;

        // Device sends ACK_OK with total size in data[1..5], not DATA
        if wrrq_response.cmd != CMD_ACK_OK || wrrq_response.data.len() < 5 {
            return Err(ZkError::InvalidResponse(format!(
                "Expected CMD_ACK_OK ({CMD_ACK_OK}) after DATA_WRRQ, got cmd={} data_len={}",
                wrrq_response.cmd,
                wrrq_response.data.len()
            )));
        }

        // Total size is at data[1..5] in little-endian format
        let total_size = u32::from_le_bytes([
            wrrq_response.data[1],
            wrrq_response.data[2],
            wrrq_response.data[3],
            wrrq_response.data[4],
        ]);

        info!("Total attendance data size: {total_size} bytes");

        let mut all_data = Vec::new();
        let mut offset: u32 = 0;

        while offset < total_size {
            // Request exactly what's remaining, up to CHUNK_SIZE
            let request_size = std::cmp::min(CHUNK_SIZE, total_size - offset);

            let mut chunk_req = [0u8; 8];
            chunk_req[0..4].copy_from_slice(&offset.to_le_bytes());
            chunk_req[4..8].copy_from_slice(&request_size.to_le_bytes());

            let mut response = self.send_command(CMD_READ_CHUNK, &chunk_req)?;

            // Skip delayed ACK_OK (2000) responses from previous commands
            while response.cmd == CMD_ACK_OK {
                debug!("Skipping delayed ACK_OK (cmd=2000)");
                response = self.read_response()?;
            }

            let chunk_data = if response.cmd == CMD_DATA {
                // Got DATA directly
                response.data
            } else if response.cmd == CMD_ACK_DATA {
                // Got ACK_DATA (1500) first, read DATA next
                let data_response = self.read_response()?;
                if data_response.cmd != CMD_DATA {
                    return Err(ZkError::InvalidResponse(format!(
                        "Expected CMD_DATA ({CMD_DATA}) after ACK_DATA, got {}",
                        data_response.cmd
                    )));
                }
                data_response.data
            } else {
                return Err(ZkError::InvalidResponse(format!(
                    "Expected CMD_DATA ({CMD_DATA}) or CMD_ACK_DATA ({CMD_ACK_DATA}), got {}",
                    response.cmd
                )));
            };

            let chunk_len = chunk_data.len() as u32;
            debug!("Read chunk: offset={offset}, requested={request_size}, received={chunk_len} bytes");

            all_data.extend_from_slice(&chunk_data);
            offset += chunk_len;
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
