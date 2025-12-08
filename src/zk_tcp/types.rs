//! ZKTeco protocol types and constants.

// Command codes
pub(crate) const CMD_CONNECT: u16 = 1000;
pub(crate) const CMD_EXIT: u16 = 1001;
pub(crate) const CMD_DISABLEDEVICE: u16 = 1003;
pub(crate) const CMD_ENABLEDEVICE: u16 = 1004;
pub(crate) const CMD_GET_FREE_SIZES: u16 = 50;
pub(crate) const CMD_ATTLOG_RRQ: u16 = 13;

// Data transfer commands
pub(crate) const CMD_PREPARE_DATA: u16 = 1500;
pub(crate) const CMD_DATA: u16 = 1501;
pub(crate) const CMD_FREE_DATA: u16 = 1502;
pub(crate) const CMD_PREPARE_BUFFER: u16 = 1503;
pub(crate) const CMD_READ_BUFFER: u16 = 1504;

// Response codes
pub(crate) const CMD_ACK_OK: u16 = 2000;

// Protocol constants
pub(crate) const TCP_HEADER: [u8; 4] = [0x50, 0x50, 0x82, 0x7D];
pub(crate) const HEADER_SIZE: usize = 8;
pub(crate) const PAYLOAD_MIN_SIZE: usize = 8; // cmd(2) + checksum(2) + session(2) + reply(2)
pub(crate) const MAX_CHUNK: usize = 0xFFC0; // ~65KB per chunk for TCP

/// Response from device including metadata.
pub(crate) struct TcpResponse {
    /// Response code from device
    pub code: u16,
    /// Payload data (after 8-byte header)
    pub data: Vec<u8>,
    /// Expected TCP payload length from header
    pub tcp_length: usize,
}
