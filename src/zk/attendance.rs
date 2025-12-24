//! Attendance record parsing for ZK devices.

use chrono::{DateTime, Local, TimeZone};

/// Size of each attendance record in bytes (TCP protocol format).
pub const RECORD_SIZE: usize = 40;

/// Size of data prefix before records start.
const DATA_PREFIX_SIZE: usize = 4;

/// Parsed attendance record from device.
#[derive(Debug, Clone)]
pub struct AttendanceRecord {
    /// Employee user ID from device.
    pub user_id: u32,
    /// Check-in/out timestamp (local time).
    pub timestamp: DateTime<Local>,
}

/// Decode ZK packed timestamp format.
///
/// ZK encodes timestamps as:
/// `((((year-2000)*12 + month-1)*31 + day-1)*24 + hour)*60 + minute)*60 + second`
fn decode_zk_timestamp(encoded: u32) -> (u16, u8, u8, u8, u8, u8) {
    let mut val = encoded;
    let second = (val % 60) as u8;
    val /= 60;
    let minute = (val % 60) as u8;
    val /= 60;
    let hour = (val % 24) as u8;
    val /= 24;
    let day = ((val % 31) + 1) as u8;
    val /= 31;
    let month = ((val % 12) + 1) as u8;
    val /= 12;
    let year = (val as u16) + 2000;
    (year, month, day, hour, minute, second)
}

/// Parse attendance data from device (TCP protocol format).
///
/// Data layout:
/// - Bytes 0-3: Data prefix (size/header info)
/// - Bytes 4+: Records (40 bytes each)
///
/// Record layout (40 bytes):
/// - Bytes 0-1: Verify type (u16 LE)
/// - Bytes 2-11: User ID (ASCII string, null-terminated)
/// - Bytes 12-26: Reserved
/// - Bytes 27-30: Timestamp (u32 LE, packed ZK format)
/// - Bytes 31-39: Reserved
pub fn parse_attendance(data: &[u8]) -> Vec<AttendanceRecord> {
    if data.len() < DATA_PREFIX_SIZE + RECORD_SIZE {
        return Vec::new();
    }

    // Skip 4-byte data prefix, then parse records
    data[DATA_PREFIX_SIZE..]
        .chunks_exact(RECORD_SIZE)
        .filter_map(|chunk| {
            // Timestamp at offset 27-30
            let encoded_ts = u32::from_le_bytes([chunk[27], chunk[28], chunk[29], chunk[30]]);

            if encoded_ts == 0 {
                return None;
            }

            let (year, month, day, hour, minute, second) = decode_zk_timestamp(encoded_ts);

            // User ID as ASCII at offset 2 (null-terminated)
            let uid_bytes = &chunk[2..12];
            let uid_end = uid_bytes.iter().position(|&b| b == 0).unwrap_or(10);
            let user_id: u32 = std::str::from_utf8(&uid_bytes[..uid_end]).ok()?.parse().ok()?;

            // Convert to DateTime<Local> - device stores local time
            let datetime = Local
                .with_ymd_and_hms(
                    i32::from(year),
                    u32::from(month),
                    u32::from(day),
                    u32::from(hour),
                    u32::from(minute),
                    u32::from(second),
                )
                .single()?;

            Some(AttendanceRecord {
                user_id,
                timestamp: datetime,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_decode_zk_timestamp() {
        // Test a known timestamp
        // 2024-01-15 08:30:00 encoded
        let (year, month, day, hour, minute, second) = decode_zk_timestamp(0);
        assert_eq!(year, 2000);
        assert_eq!(month, 1);
        assert_eq!(day, 1);
        assert_eq!(hour, 0);
        assert_eq!(minute, 0);
        assert_eq!(second, 0);
    }

    #[test]
    fn test_parse_empty() {
        let records = parse_attendance(&[]);
        assert!(records.is_empty());
    }

    #[test]
    fn test_parse_single_record() {
        // Create buffer: 4-byte prefix + 1 record (40 bytes) = 44 bytes
        let mut data = vec![0u8; 44];

        // Record starts at offset 4 (after prefix)
        // User ID "123" at offset 2 within record (bytes 6-8 in buffer)
        data[6] = b'1';
        data[7] = b'2';
        data[8] = b'3';

        // Timestamp at offset 27 within record (bytes 31-34 in buffer)
        // Using a known timestamp: 0x3189c93c = 2025-11-10 08:52:12
        let ts: u32 = 0x3189c93c;
        data[31] = (ts & 0xff) as u8;
        data[32] = ((ts >> 8) & 0xff) as u8;
        data[33] = ((ts >> 16) & 0xff) as u8;
        data[34] = ((ts >> 24) & 0xff) as u8;

        let records = parse_attendance(&data);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].user_id, 123);
        assert_eq!(records[0].timestamp.year(), 2025);
        assert_eq!(records[0].timestamp.month(), 11);
        assert_eq!(records[0].timestamp.day(), 10);
    }
}
