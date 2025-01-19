use anyhow::Result;
use macaddr::MacAddr6;
use tokio::process::{Child, Command};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug)]
struct Device {
    addr: MacAddr6,
    name: String,
    rssi: i32,
    rssis: Vec<i32>,
    uuids: Vec<Uuid>,
    paired: bool,
    trusted: bool,
    connected: bool,
}

#[derive(Debug)]
pub struct ScanResult {
    addr: MacAddr6,
    name: String,
}

impl TryFrom<&str> for ScanResult {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        // Device 00:11:22:33:44:55 My Device
        let mut split = s.trim().split_whitespace();
        let _ = split.next().unwrap();
        let addr_str = split.next().unwrap();
        let mut name = split.collect::<Vec<&str>>().join(" ");
        if name.replace("-", "") == addr_str.replace(":", "") {
            name = "".to_string();
        }
        println!("Name: {:?}", name);
        println!("Addr: {:?}", addr_str);
        let addr = addr_str.parse()?;

        Ok(Self { addr, name })
    }
}

pub enum BluetoothEvent {
    Scan(Vec<ScanResult>),
}

#[derive(Debug)]
pub struct BluetoothManager {
    channel: tokio::sync::mpsc::Sender<BluetoothEvent>,
    log_channel: tokio::sync::mpsc::Sender<String>,
    scan_process: Option<Child>,
}

impl BluetoothManager {
    pub async fn new(
        channel: tokio::sync::mpsc::Sender<BluetoothEvent>,
        log_channel: tokio::sync::mpsc::Sender<String>,
    ) -> Result<Self> {
        Command::new("bluetoothctl")
            .arg("agent")
            .arg("on")
            .output()
            .await?;
        Command::new("bluetoothctl")
            .arg("pairable")
            .arg("on")
            .output()
            .await?;

        Ok(Self {
            channel,
            log_channel,
            scan_process: None,
        })
    }

    pub async fn start_scan(&mut self) -> Result<()> {
        if self.scan_process.is_none() {
            Command::new("bluetoothctl")
                .arg("discoverable")
                .arg("on")
                .kill_on_drop(true)
                .spawn()?;
            let scan_process = Command::new("bluetoothctl").arg("scan").arg("on").spawn()?;
            self.scan_process = Some(scan_process);
        }
        Ok(())
    }

    pub async fn stop_scan(&mut self) -> Result<()> {
        if let Some(scan_process) = self.scan_process.as_mut() {
            scan_process.kill().await?;
            self.scan_process = None;
        }
        Ok(())
    }

    pub async fn get_devices(&mut self) -> Result<()> {
        let output = Command::new("bluetoothctl").arg("devices").output().await?;
        self.log_channel
            .send(String::from_utf8_lossy(&output.stdout).to_string())
            .await?;
        let mut result = vec![];
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(s) = ScanResult::try_from(line) {
                result.push(s);
            }
        }
        self.channel.send(BluetoothEvent::Scan(result)).await?;
        Ok(())
    }
}

impl Drop for BluetoothManager {
    fn drop(&mut self) {
        if let Some(scan_process) = self.scan_process.as_mut() {
            scan_process.start_kill().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_result() {
        let s = "Device 00:11:22:33:44:55 My Device";
        let result = ScanResult::try_from(s).unwrap();
        assert_eq!(result.addr.to_string(), "00:11:22:33:44:55");
        assert_eq!(result.name, "My Device");
    }
}
