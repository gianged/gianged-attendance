//! ZKTeco protocol packet building and checksum calculation.

use super::types::{HEADER_SIZE, PAYLOAD_MIN_SIZE, TCP_HEADER, TcpResponse};
use crate::error::{AppError, Result};

/// Calculate ZKTeco checksum (16-bit ones complement).
pub(crate) fn calculate_checksum(data: &[u8]) -> u16 {
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
///
/// Increments `reply_id` after building the packet.
pub(crate) fn build_packet(command: u16, data: &[u8], session_id: u16, reply_id: &mut u16) -> Vec<u8> {
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
    packet.extend_from_slice(&session_id.to_le_bytes());

    // Reply ID (2 bytes LE)
    packet.extend_from_slice(&reply_id.to_le_bytes());

    // Data
    packet.extend_from_slice(data);

    // Calculate and insert checksum (bytes 10-11, covering payload starting at byte 8)
    let checksum = calculate_checksum(&packet[8..]);
    packet[10..12].copy_from_slice(&checksum.to_le_bytes());

    // Increment reply ID for next packet
    *reply_id = reply_id.wrapping_add(1);

    packet
}

/// Extract data size from PREPARE_DATA response.
pub(crate) fn get_data_size_from_response(response: &TcpResponse) -> Result<usize> {
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
