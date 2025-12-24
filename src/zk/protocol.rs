//! ZK TCP protocol packet building and parsing.

use super::error::{Result, ZkError};

/// ZK protocol header bytes.
pub const HEADER: [u8; 4] = [0x50, 0x50, 0x82, 0x7d];

/// Maximum chunk size for data transfer (65,472 bytes).
pub const CHUNK_SIZE: u32 = 65472;

// Command codes
pub const CMD_CONNECT: u16 = 1000;
pub const CMD_EXIT: u16 = 1001;
pub const CMD_GET_FREE_SIZES: u16 = 50;
pub const CMD_ACK_OK: u16 = 2000; // General device ACK (0x07d0)
pub const CMD_ACK_DATA: u16 = 1500; // Data transfer ACK (0x05dc)
pub const CMD_DATA: u16 = 1501; // Data response (0x05dd)
pub const CMD_FREE_DATA: u16 = 1502;
pub const CMD_DATA_WRRQ: u16 = 1503;
pub const CMD_READ_CHUNK: u16 = 1504;

/// ATTLOG table identifier for data request.
pub const TABLE_ATTLOG: [u8; 11] = [0x01, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// Parsed response from device.
#[derive(Debug)]
#[allow(dead_code)]
pub struct Response {
    pub cmd: u16,
    pub session_id: u16,
    pub reply_id: u16,
    pub data: Vec<u8>,
}

/// Calculate ZK protocol checksum.
///
/// Processes data as little-endian u16 pairs, sums them,
/// folds to 16 bits, and returns one's complement.
pub fn calc_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for chunk in data.chunks(2) {
        let val = if chunk.len() == 2 {
            u16::from_le_bytes([chunk[0], chunk[1]]) as u32
        } else {
            chunk[0] as u32
        };
        sum = sum.wrapping_add(val);
    }
    // Fold to 16 bits
    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    (!sum & 0xFFFF) as u16
}

/// Build a ZK protocol packet.
///
/// Packet structure:
/// - Header (4 bytes): 50 50 82 7d
/// - Payload size (4 bytes, LE)
/// - Inner packet: cmd(2) + checksum(2) + session(2) + reply(2) + data
pub fn build_packet(cmd: u16, session_id: u16, reply_id: u16, data: &[u8]) -> Vec<u8> {
    let inner_size = 8 + data.len();

    // Build inner packet
    let mut inner = Vec::with_capacity(inner_size);
    inner.extend_from_slice(&cmd.to_le_bytes());
    inner.extend_from_slice(&[0, 0]); // checksum placeholder
    inner.extend_from_slice(&session_id.to_le_bytes());
    inner.extend_from_slice(&reply_id.to_le_bytes());
    inner.extend_from_slice(data);

    // Calculate checksum over: cmd + session + reply + data (skip checksum bytes)
    let mut chk_data = Vec::new();
    chk_data.extend_from_slice(&cmd.to_le_bytes());
    chk_data.extend_from_slice(&session_id.to_le_bytes());
    chk_data.extend_from_slice(&reply_id.to_le_bytes());
    chk_data.extend_from_slice(data);
    let checksum = calc_checksum(&chk_data);

    inner[2..4].copy_from_slice(&checksum.to_le_bytes());

    // Build full packet with header
    let mut packet = Vec::with_capacity(8 + inner_size);
    packet.extend_from_slice(&HEADER);
    packet.extend_from_slice(&(inner_size as u32).to_le_bytes());
    packet.extend_from_slice(&inner);

    packet
}

/// Parse a response packet from device.
///
/// Validates header and extracts command, session, reply ID, and data.
#[allow(dead_code)]
pub fn parse_response(packet: &[u8]) -> Result<Response> {
    if packet.len() < 8 {
        return Err(ZkError::InvalidResponse("Packet too small".to_string()));
    }

    // Verify header
    if packet[0..4] != HEADER {
        return Err(ZkError::InvalidResponse("Invalid header".to_string()));
    }

    let payload_size = u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]) as usize;

    if packet.len() < 8 + payload_size {
        return Err(ZkError::InvalidResponse(format!(
            "Incomplete packet: expected {}, got {}",
            8 + payload_size,
            packet.len()
        )));
    }

    let payload = &packet[8..8 + payload_size];

    if payload.len() < 8 {
        return Err(ZkError::InvalidResponse("Payload too small".to_string()));
    }

    Ok(Response {
        cmd: u16::from_le_bytes([payload[0], payload[1]]),
        session_id: u16::from_le_bytes([payload[4], payload[5]]),
        reply_id: u16::from_le_bytes([payload[6], payload[7]]),
        data: payload[8..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_connect() {
        // CMD_CONNECT with session=0, reply=0, no data
        // Expected checksum from capture: 0xfc17
        let data = [0xe8, 0x03, 0x00, 0x00, 0x00, 0x00]; // cmd + session + reply
        let checksum = calc_checksum(&data);
        assert_eq!(checksum, 0xfc17);
    }

    #[test]
    fn test_build_packet_connect() {
        let packet = build_packet(CMD_CONNECT, 0, 0, &[]);
        assert_eq!(&packet[0..4], &HEADER);
        assert_eq!(packet[4], 8); // inner size
        assert_eq!(u16::from_le_bytes([packet[8], packet[9]]), CMD_CONNECT);
    }
}
