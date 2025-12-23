# ZK TCP Protocol - Captured from Your Device

Based on Wireshark capture of pyzk communicating with your ZKTeco device.

## Key Discovery: Your Device Uses TCP, Not UDP

```
Device: 192.168.90.11:4370 (TCP)
Protocol: ZK TCP variant
Session ID assigned: 0xb14e
Chunk size: 65472 bytes (0xffc0)
Total attendance data: 615,124 bytes (~15,000 records)
```

---

## Packet Structure

```
┌──────────┬──────────────┬─────────────────────────────────────────┐
│  Header  │ Payload Size │              Inner Packet               │
│ 4 bytes  │   4 bytes    │              Variable                   │
├──────────┼──────────────┼──────────────────────────────────────────┤
│ 50508274 │  LE uint32   │ cmd(2) + chksum(2) + sess(2) + reply(2) + data │
└──────────┴──────────────┴─────────────────────────────────────────┘

All multi-byte values are LITTLE ENDIAN.
```

---

## Complete Command Sequence (from your capture)

### 1. CONNECT

```
TX: 5050827d 08000000 e80317fc00000000
    ^^^^^^^^ ^^^^^^^^ ^^^^^^^^^^^^^^^^
    header   size=8   cmd=1000, chk=fc17, sess=0, reply=0

RX: Device returns session ID in response (0xb14e in your case)
```

### 2. GET DEVICE INFO (x2)

```
TX: 5050827d 08000000 32007e4e 4eb1 0100
                      cmd=50   sess reply=1

TX: 5050827d 08000000 32007d4e 4eb1 0200
                      cmd=50   sess reply=2
```

### 3. PREPARE ATTLOG READ (CMD 1503)

```
TX: 5050827d 13000000 df05ce3a 4eb1 0300 010d000000000000000000
                      cmd=1503 sess reply  ^^ table=13 (ATTLOG)

The "010d00..." is the table identifier:
  01 = read operation
  0d = 13 = ATTLOG table
```

### 4. REQUEST FIRST CHUNK (CMD 1504)

```
TX: 5050827d 10000000 e005c924 4eb1 0400 00000000 04240000
                      cmd=1504 sess reply offset=0 size=9220

Device responds with data size available.
```

### 5. READ DATA CHUNKS (CMD 1504, repeat)

```
Chunk 0:  offset=0,      size=65472
TX: 5050827d 10000000 e00509494eb1 0700 00000000 c0ff0000

Chunk 1:  offset=65472,  size=65472
TX: 5050827d 10000000 e00547494eb1 0800 c0ff0000 c0ff0000

Chunk 2:  offset=130944, size=65472
TX: 5050827d 10000000 e00585494eb1 0900 80ff0100 c0ff0000

... continue until all data received ...

Last chunk: offset=589248, size=25876
TX: 5050827d 10000000 e005e3e54eb1 1000 c0fd0800 14650000
```

### 6. FREE DATA BUFFER (CMD 1502)

```
TX: 5050827d 08000000 de05c248 4eb1 1100
                      cmd=1502 sess reply=17
```

### 7. DISCONNECT (CMD 1001)

```
TX: 5050827d 08000000 e903b64a 4eb1 1200
                      cmd=1001 sess reply=18
```

---

## Command Reference

| Command            | Code          | Description         |
| ------------------ | ------------- | ------------------- |
| CMD_CONNECT        | 1000 (0x03e8) | Start session       |
| CMD_EXIT           | 1001 (0x03e9) | End session         |
| CMD_GET_FREE_SIZES | 50 (0x0032)   | Device info/stats   |
| CMD_FREE_DATA      | 1502 (0x05de) | Release data buffer |
| CMD_DATA_WRRQ      | 1503 (0x05df) | Prepare table read  |
| CMD_READ_CHUNK     | 1504 (0x05e0) | Read data chunk     |

---

## Rust Implementation

```rust
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const HEADER: [u8; 4] = [0x50, 0x50, 0x82, 0x7d];
const CHUNK_SIZE: u32 = 65472; // 0xffc0

const CMD_CONNECT: u16 = 1000;
const CMD_EXIT: u16 = 1001;
const CMD_GET_FREE_SIZES: u16 = 50;
const CMD_FREE_DATA: u16 = 1502;
const CMD_DATA_WRRQ: u16 = 1503;
const CMD_READ_CHUNK: u16 = 1504;

pub struct ZkClient {
    stream: TcpStream,
    session_id: u16,
    reply_id: u16,
}

impl ZkClient {
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        let mut client = Self {
            stream,
            session_id: 0,
            reply_id: 0,
        };

        // Send connect command
        let response = client.send_command(CMD_CONNECT, &[])?;
        client.session_id = response.session_id;

        Ok(client)
    }

    pub fn disconnect(&mut self) -> std::io::Result<()> {
        self.send_command(CMD_EXIT, &[])?;
        Ok(())
    }

    pub fn get_attendance(&mut self) -> std::io::Result<Vec<u8>> {
        // Get device info first (seems required)
        self.send_command(CMD_GET_FREE_SIZES, &[])?;
        self.send_command(CMD_GET_FREE_SIZES, &[])?;

        // Prepare ATTLOG read: 01 0d 00 00 00 00 00 00 00 00 00
        let table_data = [0x01, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        self.send_command(CMD_DATA_WRRQ, &table_data)?;

        // Get total size (first chunk request with size query)
        let size_query = [0x00, 0x00, 0x00, 0x00, 0x04, 0x24, 0x00, 0x00];
        let response = self.send_command(CMD_READ_CHUNK, &size_query)?;

        // Parse total data size from response
        let total_size = if response.data.len() >= 4 {
            u32::from_le_bytes([response.data[0], response.data[1], response.data[2], response.data[3]])
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "No size"));
        };

        // Free this initial buffer
        self.send_command(CMD_FREE_DATA, &[])?;

        // Now read actual attendance data
        // Prepare again
        self.send_command(CMD_DATA_WRRQ, &table_data)?;

        let mut all_data = Vec::with_capacity(total_size as usize);
        let mut offset: u32 = 0;

        while offset < total_size {
            let remaining = total_size - offset;
            let chunk_size = remaining.min(CHUNK_SIZE);

            let mut chunk_req = [0u8; 8];
            chunk_req[0..4].copy_from_slice(&offset.to_le_bytes());
            chunk_req[4..8].copy_from_slice(&chunk_size.to_le_bytes());

            let response = self.send_command(CMD_READ_CHUNK, &chunk_req)?;
            all_data.extend_from_slice(&response.data);

            offset += chunk_size;
        }

        // Free buffer
        self.send_command(CMD_FREE_DATA, &[])?;

        Ok(all_data)
    }

    fn send_command(&mut self, cmd: u16, data: &[u8]) -> std::io::Result<Response> {
        let packet = self.build_packet(cmd, data);
        self.stream.write_all(&packet)?;
        self.reply_id += 1;

        self.read_response()
    }

    fn build_packet(&self, cmd: u16, data: &[u8]) -> Vec<u8> {
        // Inner packet: cmd(2) + checksum(2) + session(2) + reply(2) + data
        let inner_size = 8 + data.len();

        let mut inner = Vec::with_capacity(inner_size);
        inner.extend_from_slice(&cmd.to_le_bytes());
        inner.extend_from_slice(&[0, 0]); // checksum placeholder
        inner.extend_from_slice(&self.session_id.to_le_bytes());
        inner.extend_from_slice(&self.reply_id.to_le_bytes());
        inner.extend_from_slice(data);

        // Calculate checksum over: cmd + session + reply + data (skip checksum bytes)
        let mut chk_data = Vec::new();
        chk_data.extend_from_slice(&cmd.to_le_bytes());
        chk_data.extend_from_slice(&self.session_id.to_le_bytes());
        chk_data.extend_from_slice(&self.reply_id.to_le_bytes());
        chk_data.extend_from_slice(data);
        let checksum = calc_checksum(&chk_data);

        inner[2..4].copy_from_slice(&checksum.to_le_bytes());

        // Build full packet
        let mut packet = Vec::with_capacity(8 + inner_size);
        packet.extend_from_slice(&HEADER);
        packet.extend_from_slice(&(inner_size as u32).to_le_bytes());
        packet.extend_from_slice(&inner);

        packet
    }

    fn read_response(&mut self) -> std::io::Result<Response> {
        // Read header (8 bytes)
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header)?;

        // Verify header
        if &header[0..4] != &HEADER {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Bad header"));
        }

        let payload_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;

        // Read payload
        let mut payload = vec![0u8; payload_size];
        self.stream.read_exact(&mut payload)?;

        // Parse inner packet
        if payload.len() >= 8 {
            let cmd = u16::from_le_bytes([payload[0], payload[1]]);
            let session_id = u16::from_le_bytes([payload[4], payload[5]]);
            let reply_id = u16::from_le_bytes([payload[6], payload[7]]);
            let data = payload[8..].to_vec();

            Ok(Response { cmd, session_id, reply_id, data })
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Payload too small"))
        }
    }
}

fn calc_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for chunk in data.chunks(2) {
        let val = if chunk.len() == 2 {
            u16::from_le_bytes([chunk[0], chunk[1]]) as u32
        } else {
            chunk[0] as u32
        };
        sum = sum.wrapping_add(val);
    }
    while sum > 0xFFFF {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    (!sum & 0xFFFF) as u16
}

struct Response {
    cmd: u16,
    session_id: u16,
    reply_id: u16,
    data: Vec<u8>,
}

impl Drop for ZkClient {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}
```

---

## Attendance Record Format

**Verified from Wireshark capture: 40-byte records**

Total data: 615,217 bytes → ~15,379 records (matches pyzk output)

```
Offset  Size  Field
------  ----  -----
0-11    12    Reserved (zeros, byte 11 sometimes has value)
12-15   4     Timestamp (u32 LE, ZK encoded format - see below!)
16-23   8     Reserved (zeros)
24-26   3     Unknown (byte 25 has varying values)
27-34   8     User ID as ASCII string (null-padded, e.g. "62", "177")
35-39   5     Reserved (zeros)
```

**CRITICAL: ZK Timestamp Format**

ZK does NOT use seconds-since-epoch. It uses a packed format:

```
encoded = ((((year-2000)*12 + month-1)*31 + day-1)*24 + hour)*60 + minute)*60 + second
```

Decode function:

```rust
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
```

Parse function:

```rust
fn parse_attendance(data: &[u8]) -> Vec<AttendanceRecord> {
    const RECORD_SIZE: usize = 40;

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
            let user_id: u32 = std::str::from_utf8(&uid_bytes[..uid_end])
                .ok()?
                .parse()
                .ok()?;

            // Convert to Unix timestamp for storage
            use chrono::{TimeZone, Utc};
            let datetime = Utc.with_ymd_and_hms(year as i32, month as u32, day as u32,
                                                 hour as u32, minute as u32, second as u32)
                .single()?;

            Some(AttendanceRecord {
                user_id,
                timestamp: datetime.timestamp(),
            })
        })
        .collect()
}
```

---

## Testing

To verify the Rust implementation matches pyzk:

1. Run Rust client, capture with Wireshark
2. Compare packet bytes with the known-good sequence above
3. If bytes differ, the checksum calculation is likely the issue

---

## Summary

| Item         | Value                                                                 |
| ------------ | --------------------------------------------------------------------- |
| Protocol     | TCP (not UDP)                                                         |
| Port         | 4370                                                                  |
| Header       | `50 50 82 7d`                                                         |
| Chunk size   | 65,472 bytes                                                          |
| Record size  | 40 bytes                                                              |
| Key commands | 1000 (connect), 1503 (prepare), 1504 (read), 1502 (free), 1001 (exit) |
