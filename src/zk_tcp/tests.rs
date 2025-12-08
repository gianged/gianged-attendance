//! Unit tests for ZKTeco TCP client.

use super::client::ZkTcpClient;
use super::parser::parse_attendance_data;
use super::protocol::calculate_checksum;
use super::types::{CMD_CONNECT, HEADER_SIZE, PAYLOAD_MIN_SIZE, TCP_HEADER};

#[test]
fn test_calculate_checksum() {
    // Test with known data (CMD_CONNECT packet payload with zeros)
    let data = [0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let checksum = calculate_checksum(&data);

    // Checksum should be non-zero for this data
    assert!(checksum > 0);
}

#[test]
fn test_checksum_empty_data() {
    let data: [u8; 0] = [];
    let checksum = calculate_checksum(&data);
    assert_eq!(checksum, 0xFFFF); // Complement of 0
}

#[test]
fn test_build_packet_structure() {
    let mut client = ZkTcpClient::new("127.0.0.1", 4370, 30);
    let packet = client.build_packet_for_test(CMD_CONNECT, &[]);

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
    let packet = client.build_packet_for_test(CMD_CONNECT, &extra_data);

    // Verify payload length includes extra data
    let payload_len = u32::from_le_bytes([packet[4], packet[5], packet[6], packet[7]]);
    assert_eq!(payload_len as usize, PAYLOAD_MIN_SIZE + extra_data.len());

    // Verify extra data is at the end
    assert_eq!(&packet[16..20], &extra_data);
}

#[test]
fn test_parse_text_format() {
    let data = b"20\t\t2025-12-02 07:36:58\t2\t0\n65\t\t2025-12-02 08:15:23\t2\t0\n";

    let records = parse_attendance_data(data).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].scanner_uid, 20);
    assert_eq!(records[0].verify_type, 2);
    assert_eq!(records[0].status, 0);
    assert_eq!(records[1].scanner_uid, 65);
}

#[test]
fn test_parse_text_format_skip_invalid() {
    let data = b"invalid\t\t2025-12-02 07:36:58\t2\t0\n20\t\t2025-12-02 08:15:23\t2\t0\n";

    let records = parse_attendance_data(data).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].scanner_uid, 20);
}

#[test]
fn test_parse_binary_format() {
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

    let records = parse_attendance_data(&record).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].scanner_uid, 20);
    assert_eq!(records[0].verify_type, 2);
    assert_eq!(records[0].status, 0);
}

#[test]
fn test_parse_binary_format_multiple_records() {
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

    let records = parse_attendance_data(&data).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].scanner_uid, 20);
    assert_eq!(records[1].scanner_uid, 65);
}

#[test]
fn test_reply_id_increments() {
    let mut client = ZkTcpClient::new("127.0.0.1", 4370, 30);
    assert_eq!(client.reply_id(), 0);

    let _ = client.build_packet_for_test(CMD_CONNECT, &[]);
    assert_eq!(client.reply_id(), 1);

    let _ = client.build_packet_for_test(super::types::CMD_EXIT, &[]);
    assert_eq!(client.reply_id(), 2);
}
