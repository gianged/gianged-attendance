//! Attendance data format parsers (text/binary).

use crate::error::{AppError, Result};
use crate::models::attendance::CreateAttendanceLog;
use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use tracing::warn;

/// Parse attendance data (auto-detect format).
pub(crate) fn parse_attendance_data(data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
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
        parse_text_format(data)
    } else {
        parse_binary_format(data)
    }
}

/// Parse text (TSV) format attendance data.
fn parse_text_format(data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
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
            Err(_) => {
                warn!("Invalid scanner_uid in line: {line}");
                continue;
            }
        };

        let timestamp_str = parts[2].trim();
        let check_time = match parse_local_timestamp(timestamp_str) {
            Ok(dt) => dt,
            Err(_) => {
                warn!("Invalid timestamp in line: {line}");
                continue;
            }
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
fn parse_binary_format(data: &[u8]) -> Result<Vec<CreateAttendanceLog>> {
    const RECORD_SIZE: usize = 40;
    let mut records = Vec::new();

    for chunk in data.chunks_exact(RECORD_SIZE) {
        // Extract user ID (offset 0-9, null-terminated string)
        let user_id_end = chunk[0..9].iter().position(|&b| b == 0).unwrap_or(9);
        let user_id_str = match std::str::from_utf8(&chunk[0..user_id_end]) {
            Ok(s) => s.trim(),
            Err(_) => {
                warn!("Invalid UTF-8 in user ID field");
                continue;
            }
        };

        let scanner_uid = match user_id_str.parse::<i32>() {
            Ok(uid) => uid,
            Err(_) => {
                warn!("Invalid scanner_uid: {user_id_str}");
                continue;
            }
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
            None => {
                warn!("Ambiguous local time for timestamp: {timestamp_raw}");
                continue;
            }
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
fn parse_local_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>> {
    let naive_dt = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| AppError::parse(format!("Invalid timestamp '{timestamp_str}': {e}")))?;

    let local_dt = Local
        .from_local_datetime(&naive_dt)
        .single()
        .ok_or_else(|| AppError::parse(format!("Ambiguous local time: {timestamp_str}")))?;

    Ok(local_dt.with_timezone(&Utc))
}
