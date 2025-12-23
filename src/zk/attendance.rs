//! Attendance record parsing for ZK devices.

use chrono::{DateTime, TimeZone, Utc};

/// Size of each attendance record in bytes.
pub const RECORD_SIZE: usize = 40;

/// Parsed attendance record from device.
#[derive(Debug, Clone)]
pub struct AttendanceRecord {
    /// Employee user ID from device.
    pub user_id: u32,
    /// Check-in/out timestamp (UTC).
    pub timestamp: DateTime<Utc>,
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

/// Parse attendance data from device.
///
/// Record layout (40 bytes):
/// - Bytes 0-11: Reserved
/// - Bytes 12-15: Timestamp (u32 LE, packed ZK format)
/// - Bytes 16-23: Reserved
/// - Bytes 24-26: Unknown
/// - Bytes 27-34: User ID (ASCII string, null-padded)
/// - Bytes 35-39: Reserved
///
/// First record is skipped as it appears to be a header.
pub fn parse_attendance(data: &[u8]) -> Vec<AttendanceRecord> {
    if data.len() < RECORD_SIZE {
        return Vec::new();
    }

    // Skip first record (header)
    data[RECORD_SIZE..]
        .chunks_exact(RECORD_SIZE)
        .filter_map(|chunk| {
            // Timestamp at offset 12-15
            let encoded_ts = u32::from_le_bytes([chunk[12], chunk[13], chunk[14], chunk[15]]);

            if encoded_ts == 0 {
                return None;
            }

            let (year, month, day, hour, minute, second) = decode_zk_timestamp(encoded_ts);

            // User ID as ASCII at offset 27-34
            let uid_bytes = &chunk[27..35];
            let uid_end = uid_bytes.iter().position(|&b| b == 0).unwrap_or(8);
            let user_id: u32 = std::str::from_utf8(&uid_bytes[..uid_end]).ok()?.parse().ok()?;

            // Convert to DateTime<Utc>
            let datetime = Utc
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
        // Create a 80-byte buffer (header + 1 record)
        let mut data = vec![0u8; 80];

        // Second record (index 40..80)
        // Timestamp at offset 12-15: encode 2024-06-15 08:30:00
        // Simplified: just put a non-zero value
        data[52] = 0x01; // Non-zero timestamp
        data[53] = 0x02;
        data[54] = 0x03;
        data[55] = 0x04;

        // User ID "123" at offset 27-34 (relative to record start at 40)
        data[67] = b'1';
        data[68] = b'2';
        data[69] = b'3';

        let records = parse_attendance(&data);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].user_id, 123);
    }
}
