//! Low-level socket I/O operations with timeout handling.

use super::types::{HEADER_SIZE, PAYLOAD_MIN_SIZE, TCP_HEADER, TcpResponse};
use crate::error::{AppError, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{error, info};

/// Write packet to stream with timeout.
pub(crate) async fn write_packet(stream: &mut TcpStream, packet: &[u8], timeout_duration: Duration) -> Result<()> {
    println!(
        "[ZK] TX ({} bytes): {:02X?}",
        packet.len(),
        &packet[..packet.len().min(32)]
    );
    timeout(timeout_duration, stream.write_all(packet))
        .await
        .map_err(|_| AppError::DeviceTimeout("Write timeout".to_string()))?
        .map_err(|e| {
            error!("Write failed: {e}");
            AppError::TcpConnectionFailed(format!("Write failed: {e}"))
        })?;
    println!("[ZK] TX complete");
    Ok(())
}

/// Read a response packet with full metadata.
pub(crate) async fn read_response(stream: &mut TcpStream, timeout_duration: Duration) -> Result<TcpResponse> {
    // Read TCP header (8 bytes: magic + length)
    println!("[ZK] RX waiting for header ({:?} timeout)...", timeout_duration);
    let mut header = [0u8; HEADER_SIZE];
    timeout(timeout_duration, stream.read_exact(&mut header))
        .await
        .map_err(|_| {
            println!("[ZK] RX TIMEOUT!");
            error!("Read timeout waiting for header");
            AppError::DeviceTimeout("Read timeout".to_string())
        })?
        .map_err(|e| {
            error!("Read failed: {e}");
            AppError::TcpConnectionFailed(format!("Read failed: {e}"))
        })?;
    println!("[ZK] RX header: {:02X?}", header);

    // Verify magic bytes
    if header[0..4] != TCP_HEADER {
        error!("Invalid TCP header: {:02X?}", &header[0..4]);
        return Err(AppError::TcpProtocolError(format!(
            "Invalid TCP header: {:02X?}",
            &header[0..4]
        )));
    }

    // Extract payload length (this is tcp_length)
    let tcp_length = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

    // Safety limit
    if tcp_length > 1_000_000 {
        error!("Payload too large: {tcp_length} bytes");
        return Err(AppError::TcpProtocolError(format!(
            "Payload too large: {tcp_length} bytes"
        )));
    }

    // Read payload
    let mut payload = vec![0u8; tcp_length];
    timeout(timeout_duration, stream.read_exact(&mut payload))
        .await
        .map_err(|_| AppError::DeviceTimeout("Payload read timeout".to_string()))?
        .map_err(|e| {
            error!("Payload read failed: {e}");
            AppError::TcpConnectionFailed(format!("Payload read failed: {e}"))
        })?;

    // Verify minimum payload size
    if payload.len() < PAYLOAD_MIN_SIZE {
        error!("Payload too small: {} bytes", payload.len());
        return Err(AppError::TcpProtocolError(format!(
            "Payload too small: {} bytes",
            payload.len()
        )));
    }

    // Extract response code
    let code = u16::from_le_bytes([payload[0], payload[1]]);

    // Data is everything after the 8-byte header
    let data = if payload.len() > PAYLOAD_MIN_SIZE {
        payload[PAYLOAD_MIN_SIZE..].to_vec()
    } else {
        Vec::new()
    };

    Ok(TcpResponse { code, data, tcp_length })
}

/// Read raw data from socket (for streaming data after CMD_DATA).
pub(crate) async fn read_raw_data(stream: &mut TcpStream, size: usize, timeout_duration: Duration) -> Result<Vec<u8>> {
    let mut data = vec![0u8; size];
    let mut received = 0;

    while received < size {
        let chunk_size = (size - received).min(65536);
        match timeout(
            timeout_duration,
            stream.read(&mut data[received..received + chunk_size]),
        )
        .await
        {
            Ok(Ok(0)) => {
                // Connection closed
                break;
            }
            Ok(Ok(n)) => {
                received += n;
            }
            Ok(Err(e)) => {
                error!("Raw read failed: {e}");
                return Err(AppError::TcpConnectionFailed(format!("Raw read failed: {e}")));
            }
            Err(_) => {
                return Err(AppError::DeviceTimeout("Raw data timeout".to_string()));
            }
        }
    }

    data.truncate(received);
    Ok(data)
}
