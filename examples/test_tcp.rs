//! Test TCP connection to ZKTeco device.
//!
//! Usage: cargo run --example test_tcp [IP]
//!
//! Default IP: 192.168.90.11

use gianged_attendance::zk_tcp_client::ZkTcpClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let ip = std::env::args().nth(1).unwrap_or_else(|| "192.168.90.11".to_string());

    let port: u16 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(4370);

    println!("Testing TCP connection to {ip}:{port}");
    println!("======================================");

    let mut client = ZkTcpClient::new(&ip, port, 30);

    println!("\n[1] Connecting...");
    client.connect().await?;
    println!("    Connected! Session established.");

    println!("\n[2] Getting record count...");
    match client.get_attendance_count().await {
        Ok(count) => println!("    Device reports {count} attendance records"),
        Err(e) => println!("    Warning: Could not get record count: {e}"),
    }

    println!("\n[3] Downloading attendance data...");
    let records = client.download_attendance().await?;
    println!("    Downloaded {} records", records.len());

    if !records.is_empty() {
        println!("\n    First 5 records:");
        for (i, record) in records.iter().take(5).enumerate() {
            println!(
                "      {}. UID {:3} | {} | verify: {} | status: {}",
                i + 1,
                record.scanner_uid,
                record.check_time.format("%Y-%m-%d %H:%M:%S"),
                record.verify_type,
                record.status
            );
        }

        if records.len() > 5 {
            println!("\n    Last 5 records:");
            for (i, record) in records.iter().rev().take(5).rev().enumerate() {
                let idx = records.len() - 5 + i;
                println!(
                    "      {}. UID {:3} | {} | verify: {} | status: {}",
                    idx + 1,
                    record.scanner_uid,
                    record.check_time.format("%Y-%m-%d %H:%M:%S"),
                    record.verify_type,
                    record.status
                );
            }
        }

        // Show some stats
        println!("\n    Statistics:");
        let unique_users: std::collections::HashSet<i32> = records.iter().map(|r| r.scanner_uid).collect();
        println!("      Unique users: {}", unique_users.len());

        if let (Some(first), Some(last)) = (records.first(), records.last()) {
            println!(
                "      Date range: {} to {}",
                first.check_time.format("%Y-%m-%d"),
                last.check_time.format("%Y-%m-%d")
            );
        }
    }

    println!("\n[4] Disconnecting...");
    client.disconnect().await?;
    println!("    Disconnected.");

    println!("\n======================================");
    println!("Done!");

    Ok(())
}
