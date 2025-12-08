# ZKTeco TCP Protocol Implementation

## Problem Summary

The current Rust attendance app uses HTTP interface (`/form/Download` on port 80) to fetch attendance records from the ZKTeco fingerprint scanner. However, **the HTTP interface only returns a subset of records**.

The old ERP software showed additional records that are missing from both:

- The scanner's web UI
- The current Rust app

### Evidence

| Test                                    | Result                                        |
| --------------------------------------- | --------------------------------------------- |
| TCP port 4370                           | **OPEN** - device responded                   |
| CMD_CONNECT (1000)                      | Success - session ID 55549                    |
| CMD_GET_FREE_SIZES (50)                 | Device reports **~10,120 attendance records** |
| HTTP /form/Download                     | Returns incomplete data                       |
| Staff UID 20 on 2025-12-02 & 2025-12-04 | Missing records after ~16:00                  |

### Root Cause

ZKTeco devices have **two interfaces**:

1. **HTTP/Web Interface (port 80)** - Limited CSL web buffer, incomplete data
2. **Binary TCP Protocol (port 4370)** - Full device storage, complete data

The old ERP used TCP/4370. The current app uses HTTP/80.

---

## Solution

Implement a new TCP client (`ZkTcpClient`) that communicates via the binary protocol on port 4370.

---

## Protocol Specification

### Connection Details

- **IP**: 192.168.90.11 (configurable)
- **Port**: 4370
- **Protocol**: TCP with custom binary framing

### Packet Structure

Every TCP packet has this format:

```
[TCP Header: 4 bytes] [Payload Length: 4 bytes LE] [Payload: N bytes]
```

**TCP Header (magic bytes):**

```
0x50 0x50 0x82 0x7D
```

**Payload Structure:**

```
[Command: 2 bytes LE] [Checksum: 2 bytes LE] [Session ID: 2 bytes LE] [Reply ID: 2 bytes LE] [Data: N bytes]
```

### Checksum Algorithm

```rust
fn calculate_checksum(data: &[u8]) -> u16 {
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
    (!sum as u16) & 0xFFFF
}
```

**Important:** Calculate checksum with the checksum field set to 0, then insert the result.

### Command Codes

| Command            | Code          | Description                         |
| ------------------ | ------------- | ----------------------------------- |
| CMD_CONNECT        | 1000 (0x03E8) | Establish session                   |
| CMD_EXIT           | 1001 (0x03E9) | Close session                       |
| CMD_DISABLEDEVICE  | 1003 (0x03EB) | Lock device during data transfer    |
| CMD_ENABLEDEVICE   | 1004 (0x03EC) | Unlock device                       |
| CMD_GET_FREE_SIZES | 50 (0x0032)   | Get device statistics/record counts |
| CMD_ATTLOG_RRQ     | 13 (0x000D)   | Request attendance log              |
| CMD_DATA_RDY       | 1500 (0x05DC) | Ready to receive data chunk         |
| CMD_FREE_DATA      | 1502 (0x05DE) | Free device data buffer             |
| CMD_ACK_OK         | 2000 (0x07D0) | Acknowledgment response             |
| CMD_PREPARE_DATA   | 1500 (0x05DC) | Data ready notification             |

### Response Codes

| Response | Meaning                              |
| -------- | ------------------------------------ |
| 2000     | CMD_ACK_OK - Success                 |
| 2005     | CMD_ACK_OK (alternate)               |
| 1500     | CMD_PREPARE_DATA - Data buffer ready |

---

## Communication Flow

### 1. Connect

```
TX: CMD_CONNECT (1000), session=0
RX: CMD_ACK_OK (2000), session=<new_session_id>
```

Save the session ID from response bytes [4:6] (little-endian).

### 2. Get Record Count (Optional)

```
TX: CMD_GET_FREE_SIZES (50), session=<session_id>
RX: CMD_ACK_OK (2000), data contains device stats
```

Attendance count is at offset 32 in the response data (4 bytes LE).

### 3. Download Attendance

```
TX: CMD_DISABLEDEVICE (1003)    # Lock device
RX: CMD_ACK_OK (2000)

TX: CMD_ATTLOG_RRQ (13)         # Request attendance data
RX: CMD_PREPARE_DATA (1500)     # Response includes data size at offset 8

# Read data in chunks
LOOP:
  TX: CMD_DATA_RDY (1500)
  RX: Data chunk (up to ~64KB)
  # Continue until all data received or small response

TX: CMD_FREE_DATA (1502)        # Free buffer
RX: CMD_ACK_OK (2000)

TX: CMD_ENABLEDEVICE (1004)     # Unlock device
RX: CMD_ACK_OK (2000)
```

### 4. Disconnect

```
TX: CMD_EXIT (1001)
RX: CMD_ACK_OK (2000)
```

---

## Attendance Data Format

The attendance data can be in **text** or **binary** format depending on device firmware.

### Text Format (Tab-Separated)

```
<scanner_uid>\t\t<timestamp>\t<verify_type>\t<status>
```

Example:

```
20		2025-12-02 07:36:58	2	0
20		2025-12-02 17:45:23	2	0
```

Fields:

- `scanner_uid`: Employee's device user ID (integer)
- `timestamp`: Local time `YYYY-MM-DD HH:MM:SS`
- `verify_type`: 2=fingerprint, 101=card
- `status`: Usually 0

### Binary Format (40 bytes per record)

| Offset | Length | Field                                    |
| ------ | ------ | ---------------------------------------- |
| 0      | 9      | User ID (null-terminated string)         |
| 24     | 4      | Timestamp (seconds since 2000-01-01, LE) |
| 28     | 1      | Verify type                              |
| 29     | 1      | Status                                   |

**Timestamp conversion:**

```rust
let base = NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0);
let timestamp = base + Duration::seconds(raw_timestamp as i64);
```

---

## Implementation Plan

### Files to Create/Modify

1. **Create `src/zk_tcp_client.rs`** - New TCP client module
2. **Modify `src/lib.rs`** - Export new module
3. **Modify `src/sync.rs`** - Use TCP client for sync
4. **Modify `src/config.rs`** - Add TCP port config option

### New Struct: `ZkTcpClient`

```rust
pub struct ZkTcpClient {
    stream: Option<TcpStream>,
    session_id: u16,
    ip: String,
    port: u16,
}

impl ZkTcpClient {
    pub fn new(ip: &str, port: u16) -> Self;
    pub fn connect(&mut self) -> Result<()>;
    pub fn disconnect(&mut self) -> Result<()>;
    pub fn get_attendance_count(&mut self) -> Result<u32>;
    pub fn download_attendance(&mut self) -> Result<Vec<CreateAttendanceLog>>;

    // Private helpers
    fn send_command(&mut self, cmd: u16, data: &[u8]) -> Result<Vec<u8>>;
    fn read_data_chunks(&mut self, expected_size: usize) -> Result<Vec<u8>>;
    fn parse_attendance_data(&self, data: &[u8]) -> Result<Vec<CreateAttendanceLog>>;
    fn calculate_checksum(&self, data: &[u8]) -> u16;
}
```

### Config Addition

```toml
[device]
url = "http://192.168.90.11"
tcp_port = 4370              # NEW
use_tcp = true               # NEW - prefer TCP over HTTP
username = "administrator"
password = "123456"
```

### Sync Logic Change

```rust
// In sync.rs
pub async fn sync(&self) -> Result<SyncResult> {
    if self.config.device.use_tcp {
        // Try TCP first
        match self.sync_via_tcp().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("TCP sync failed, falling back to HTTP: {}", e);
            }
        }
    }

    // Fall back to HTTP
    self.sync_via_http().await
}
```

---

## Testing

### Test Script (PowerShell) - Verified Working

```powershell
$ip = "192.168.90.11"
$tcp = New-Object System.Net.Sockets.TcpClient
$tcp.Connect($ip, 4370)
$stream = $tcp.GetStream()

# CMD_CONNECT packet
$packet = [byte[]]@(0x50,0x50,0x82,0x7D, 0x08,0x00,0x00,0x00, 0xE8,0x03,0x17,0xFC, 0x00,0x00,0x00,0x00)
$stream.Write($packet, 0, 16)

Start-Sleep -Milliseconds 500
$buf = [byte[]]::new(1024)
$n = $stream.Read($buf, 0, 1024)

Write-Host "Response: $n bytes"
Write-Host "Session ID: $([BitConverter]::ToUInt16($buf, 12))"

$tcp.Close()
```

**Expected output:**

```
Response: 16 bytes
Session ID: 55549
```

### Rust Test Example

```rust
// examples/test_tcp.rs
use gianged_attendance::zk_tcp_client::ZkTcpClient;

fn main() -> anyhow::Result<()> {
    let mut client = ZkTcpClient::new("192.168.90.11", 4370);

    println!("Connecting...");
    client.connect()?;
    println!("Connected!");

    println!("Getting record count...");
    let count = client.get_attendance_count()?;
    println!("Device has {} attendance records", count);

    println!("Downloading attendance...");
    let records = client.download_attendance()?;
    println!("Downloaded {} records", records.len());

    // Show first/last few
    for r in records.iter().take(5) {
        println!("  UID {} at {}", r.scanner_uid, r.check_time);
    }

    client.disconnect()?;
    println!("Done!");

    Ok(())
}
```

---

## References

- ZKTeco protocol reverse-engineered from `pyzk` Python library
- Node.js `node-zklib` library
- Device model: Ronald Jack (ZKTeco compatible), Finger VX10.0 algorithm
- Firmware: ZK Web Server with CSL interface

---

## Summary

| Current                   | New                 |
| ------------------------- | ------------------- |
| HTTP port 80              | TCP port 4370       |
| `/form/Download` endpoint | Binary protocol     |
| Incomplete records        | Full device storage |
| ~partial data             | ~10,120 records     |

The TCP implementation should retrieve all attendance records that the old ERP was able to access.
