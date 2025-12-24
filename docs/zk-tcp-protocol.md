# ZK TCP Protocol Documentation

ZKTeco devices support a binary TCP protocol on port 4370 for reading attendance data directly from flash storage. This is more reliable than the HTTP interface which has buffer limitations.

## Protocol Overview

| Property | Value |
|----------|-------|
| Port | 4370 |
| Transport | TCP |
| Byte Order | Little-endian |
| Header | `50 50 82 7d` |

## Packet Structure

### Request/Response Format

```
┌────────────┬──────────────┬─────────────────────────────────────┐
│  Header    │ Payload Size │              Payload                │
│  4 bytes   │   4 bytes    │           variable                  │
└────────────┴──────────────┴─────────────────────────────────────┘
```

### Payload Structure

```
┌─────────┬──────────┬────────────┬───────────┬──────────┐
│   CMD   │ Checksum │ Session ID │ Reply ID  │   Data   │
│ 2 bytes │ 2 bytes  │  2 bytes   │  2 bytes  │ variable │
└─────────┴──────────┴────────────┴───────────┴──────────┘
```

### Checksum Calculation

```rust
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
```

## Command Codes

### Currently Implemented

| Command | Code | Description |
|---------|------|-------------|
| `CMD_CONNECT` | 1000 | Establish session |
| `CMD_EXIT` | 1001 | Disconnect |
| `CMD_CLEAR_ATTLOG` | 15 | Clear attendance records |
| `CMD_GET_FREE_SIZES` | 50 | Get device capacity info |
| `CMD_ACK_OK` | 2000 | General acknowledgment |
| `CMD_ACK_DATA` | 1500 | Data transfer acknowledgment |
| `CMD_DATA` | 1501 | Data response |
| `CMD_FREE_DATA` | 1502 | Free device buffer |
| `CMD_DATA_WRRQ` | 1503 | Prepare data read |
| `CMD_READ_CHUNK` | 1504 | Read data chunk |

### Available for Future Implementation

| Command | Code | Description |
|---------|------|-------------|
| `CMD_CLEAR_DATA` | 14 | Clear all data |
| `CMD_DELETE_USER` | 18 | Delete user |
| `CMD_DELETE_USERTEMP` | 19 | Delete fingerprint template |
| `CMD_CLEAR_ADMIN` | 20 | Clear admin privilege |
| `CMD_USERTEMP_RRQ` | 9 | Read user templates |
| `CMD_ATTLOG_RRQ` | 13 | Read attendance log (alternative) |

## CMD_GET_FREE_SIZES Response Format

The `CMD_GET_FREE_SIZES` (50) command returns device capacity information.

Response data is 80 bytes containing 20 u32 values (little-endian):

| Index | Field | Description |
|-------|-------|-------------|
| 4 | users | Current user count |
| 6 | fingers | Current fingerprint count |
| 8 | records | Current attendance record count |
| 10 | dummy | Reserved |
| 12 | cards | Current card count |
| 14 | fingers_cap | Fingerprint capacity |
| 15 | users_cap | User capacity |
| 16 | records_cap | Attendance record capacity |
| 17 | fingers_av | Available fingerprint slots |
| 18 | users_av | Available user slots |
| 19 | records_av | Available record slots |

### Parsing Example

```rust
let get_u32 = |idx: usize| -> u32 {
    let offset = idx * 4;
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
};

let records = get_u32(8);       // Current record count
let records_cap = get_u32(16);  // Max capacity
let records_av = get_u32(19);   // Available slots
```

## CMD_CLEAR_ATTLOG

The `CMD_CLEAR_ATTLOG` (15) command clears all attendance records from the device.

- Request: Empty data payload
- Response: `CMD_ACK_OK` (2000) on success

### Usage Example

```rust
let response = client.send_command(CMD_CLEAR_ATTLOG, &[])?;
if response.cmd != CMD_ACK_OK {
    return Err("Clear failed");
}
```

**Warning**: This permanently deletes all attendance data from the device. Always sync to database before clearing.

## Data Flow: Reading Attendance

```
Client                              Device
  │                                   │
  ├──── CMD_CONNECT ─────────────────►│
  │◄─── ACK_OK (session_id) ──────────┤
  │                                   │
  ├──── CMD_GET_FREE_SIZES ──────────►│
  │◄─── ACK_OK ───────────────────────┤
  │                                   │
  ├──── CMD_DATA_WRRQ (TABLE_ATTLOG) ►│
  │◄─── ACK_OK (total_size in data) ──┤
  │                                   │
  ├──── CMD_READ_CHUNK (offset, size)►│
  │◄─── ACK_DATA ─────────────────────┤
  │◄─── DATA (chunk bytes) ───────────┤
  │         ... repeat ...            │
  │                                   │
  ├──── CMD_FREE_DATA ───────────────►│
  │◄─── ACK_OK ───────────────────────┤
  │                                   │
  ├──── CMD_EXIT ────────────────────►│
  │◄─── ACK_OK ───────────────────────┤
  └───────────────────────────────────┘
```

## Attendance Record Format

After `CMD_DATA_WRRQ`, the total size is in the ACK_OK response at `data[1..5]` (little-endian u32).

Data has a 4-byte prefix, then 40-byte records:

```
Record Layout (40 bytes):
┌────────────┬─────────────┬──────────┬───────────┬──────────┐
│ Verify Type│   User ID   │ Reserved │ Timestamp │ Reserved │
│  2 bytes   │  10 bytes   │ 15 bytes │  4 bytes  │  9 bytes │
│  offset 0  │  offset 2   │ offset 12│ offset 27 │ offset 31│
└────────────┴─────────────┴──────────┴───────────┴──────────┘
```

### Timestamp Encoding

ZK encodes timestamps as a packed u32:

```
encoded = ((((year-2000)*12 + month-1)*31 + day-1)*24 + hour)*60 + minute)*60 + second
```

Decode by reversing:
```rust
let second = (val % 60) as u8; val /= 60;
let minute = (val % 60) as u8; val /= 60;
let hour = (val % 24) as u8; val /= 24;
let day = ((val % 31) + 1) as u8; val /= 31;
let month = ((val % 12) + 1) as u8; val /= 12;
let year = (val as u16) + 2000;
```

## Configuration

```toml
[device]
url = "http://192.168.90.11"  # IP extracted for TCP connection
protocol = "tcp"               # Use TCP instead of HTTP
```

The `device_ip()` method extracts the IP from the URL. Port 4370 is standard.

## Chunk-Based Reading

- Maximum chunk size: 65,472 bytes
- Request exact remaining bytes for final chunk to avoid errors
- Device returns `cmd=2001` if requesting beyond available data

## References

- [pyzk](https://github.com/fananimi/pyzk) - Python implementation
- [zk-protocol](https://github.com/adrobinoga/zk-protocol) - Protocol documentation
