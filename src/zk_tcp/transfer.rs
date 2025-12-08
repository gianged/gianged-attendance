//! Buffered data transfer orchestration (chunked reads).

use super::io::{read_raw_data, read_response, write_packet};
use super::protocol::{build_packet, get_data_size_from_response};
use super::types::{
    CMD_ACK_OK, CMD_DATA, CMD_FREE_DATA, CMD_PREPARE_BUFFER, CMD_PREPARE_DATA, CMD_READ_BUFFER, MAX_CHUNK,
    PAYLOAD_MIN_SIZE,
};
use crate::error::{AppError, Result};
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{debug, error};

/// Read data using buffered commands (for large data like attendance logs).
/// This follows pyzk's read_with_buffer implementation exactly.
pub(crate) async fn read_with_buffer(
    stream: &mut TcpStream,
    command: u16,
    session_id: u16,
    reply_id: &mut u16,
    timeout_duration: Duration,
) -> Result<Vec<u8>> {
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
    let packet = build_packet(CMD_PREPARE_BUFFER, &cmd_data, session_id, reply_id);
    write_packet(stream, &packet, timeout_duration).await?;
    let response = read_response(stream, timeout_duration).await?;

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
            let more_data = read_raw_data(stream, need, timeout_duration).await?;
            let mut result = response.data;
            result.extend_from_slice(&more_data);
            return Ok(result);
        }

        return Ok(response.data);
    }

    // CMD_PREPARE_DATA response - size is at offset 1-5 in data portion
    if response.data.len() < 5 {
        error!("PREPARE_BUFFER response data too small: {} bytes", response.data.len());
        return Err(AppError::TcpProtocolError(format!(
            "PREPARE_BUFFER response data too small: {} bytes",
            response.data.len()
        )));
    }

    let size = u32::from_le_bytes([response.data[1], response.data[2], response.data[3], response.data[4]]) as usize;

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
        debug!("Reading chunk {}/{packets} at offset {start}", i + 1);
        let chunk = read_chunk(stream, start, MAX_CHUNK as u32, session_id, reply_id, timeout_duration).await?;
        debug!("Chunk {} returned {} bytes", i + 1, chunk.len());
        all_data.extend_from_slice(&chunk);
        start += MAX_CHUNK as u32;
    }

    // Read remaining data
    if remain > 0 {
        debug!("Reading final {remain} bytes at offset {start}");
        let chunk = read_chunk(stream, start, remain as u32, session_id, reply_id, timeout_duration).await?;
        debug!("Final chunk returned {} bytes", chunk.len());
        all_data.extend_from_slice(&chunk);
    }

    // Call free_data to clean up device buffer
    debug!("Freeing device buffer");
    let free_packet = build_packet(CMD_FREE_DATA, &[], session_id, reply_id);
    let _ = write_packet(stream, &free_packet, timeout_duration).await;
    let _ = read_response(stream, timeout_duration).await;

    Ok(all_data)
}

/// Read a chunk from the device buffer.
/// Follows pyzk's __read_chunk implementation.
async fn read_chunk(
    stream: &mut TcpStream,
    start: u32,
    size: u32,
    session_id: u16,
    reply_id: &mut u16,
    timeout_duration: Duration,
) -> Result<Vec<u8>> {
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
        let packet = build_packet(CMD_READ_BUFFER, &cmd_data, session_id, reply_id);
        write_packet(stream, &packet, timeout_duration).await?;
        let response = read_response(stream, timeout_duration).await?;

        debug!(
            "READ_BUFFER response: code={}, data_len={}, tcp_length={}",
            response.code,
            response.data.len(),
            response.tcp_length
        );

        // Now receive the actual chunk data
        if let Some(data) = receive_chunk(stream, &response, timeout_duration).await? {
            return Ok(data);
        }
    }

    error!("Failed to read chunk at offset {start} after 3 retries");
    Err(AppError::TcpProtocolError(format!(
        "Failed to read chunk at offset {start} after 3 retries"
    )))
}

/// Receive chunk data after a READ_BUFFER command.
/// Follows pyzk's __recieve_chunk implementation.
async fn receive_chunk(
    stream: &mut TcpStream,
    response: &super::types::TcpResponse,
    timeout_duration: Duration,
) -> Result<Option<Vec<u8>>> {
    if response.code == CMD_DATA {
        // Data follows directly - check if we need more
        let expected_data_len = response.tcp_length.saturating_sub(PAYLOAD_MIN_SIZE);
        let have_data_len = response.data.len();

        if have_data_len < expected_data_len {
            let need = expected_data_len - have_data_len;
            let more_data = read_raw_data(stream, need, timeout_duration).await?;
            let mut result = response.data.clone();
            result.extend_from_slice(&more_data);
            return Ok(Some(result));
        }

        return Ok(Some(response.data.clone()));
    } else if response.code == CMD_PREPARE_DATA {
        // Size is in response, data follows in TCP stream
        let size = get_data_size_from_response(response)?;

        // Read data from TCP stream
        let data = receive_tcp_data(stream, response, size, timeout_duration).await?;
        return Ok(Some(data));
    }

    debug!("receive_chunk: unexpected response code {}", response.code);
    Ok(None)
}

/// Receive TCP data after PREPARE_DATA response.
async fn receive_tcp_data(
    stream: &mut TcpStream,
    response: &super::types::TcpResponse,
    size: usize,
    timeout_duration: Duration,
) -> Result<Vec<u8>> {
    let mut data = Vec::new();

    // First append any data already in the response (after 8 bytes)
    if response.data.len() > 8 {
        data.extend_from_slice(&response.data[8..]);
    }

    // Read remaining data from socket
    if data.len() < size {
        let need = size - data.len();
        let more = read_raw_data(stream, need, timeout_duration).await?;
        data.extend_from_slice(&more);
    }

    // Now read the ACK packet
    let ack_response = read_response(stream, timeout_duration).await?;
    if ack_response.code != CMD_ACK_OK {
        debug!("Expected ACK_OK but got {} after data receive", ack_response.code);
    }

    Ok(data)
}
