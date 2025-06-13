use std::{hash::Hash, process::Stdio};

use anyhow::Result;
use macaddr::MacAddr6;
use tokio::process::{Child, Command};

#[allow(dead_code)]
#[derive(Debug, Eq, Clone)]
pub struct Device {
    pub addr: MacAddr6,
    pub name: String,
    pub paired: bool,
    pub trusted: bool,
    pub connected: bool,
}

impl Hash for Device {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
    }
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl From<ScanResult> for Device {
    fn from(result: ScanResult) -> Self {
        Self {
            addr: result.addr,
            name: result.name,
            paired: false,
            trusted: false,
            connected: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub addr: MacAddr6,
    pub name: String,
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
        let addr = addr_str.parse()?;

        Ok(Self { addr, name })
    }
}

pub enum BluetoothEvent {
    Scan(Vec<Device>),
}

pub enum BluetoothRequest {
    Connect(Device),
}

#[derive(Debug)]
pub struct BluetoothManager {
    channel: tokio::sync::mpsc::Sender<BluetoothEvent>,
    #[allow(dead_code)]
    log_channel: tokio::sync::mpsc::Sender<String>,
    request_channel: tokio::sync::mpsc::Receiver<BluetoothRequest>,
    scan_process: Option<Child>,
}

impl BluetoothManager {
    pub async fn new(
        channel: tokio::sync::mpsc::Sender<BluetoothEvent>,
        log_channel: tokio::sync::mpsc::Sender<String>,
        request_channel: tokio::sync::mpsc::Receiver<BluetoothRequest>,
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
            request_channel,
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
            let scan_process = Command::new("bluetoothctl")
                .arg("scan")
                .arg("on")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .spawn()?;
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
        let all = self.devices("").await?;
        let paired = self.devices("paired").await?;
        let trusted = self.devices("trusted").await?;
        let connected = self.devices("connected").await?;

        let mut devices: Vec<Device> = all.iter().map(|d| Device::from(d.clone())).collect();
        for result in &paired {
            if let Some(device) = devices.iter_mut().find(|d| d.addr == result.addr) {
                device.paired = true;
            }
        }
        for result in &trusted {
            if let Some(device) = devices.iter_mut().find(|d| d.addr == result.addr) {
                device.trusted = true;
            }
        }
        for result in &connected {
            if let Some(device) = devices.iter_mut().find(|d| d.addr == result.addr) {
                device.connected = true;
            }
        }

        self.channel.send(BluetoothEvent::Scan(devices)).await?;

        Ok(())
    }

    /// Assumes that scanning is running in the background
    async fn devices(&mut self, filter: &str) -> Result<Vec<ScanResult>> {
        let output = if filter.is_empty() {
            Command::new("bluetoothctl").arg("devices").output().await?
        } else {
            Command::new("bluetoothctl")
                .arg("devices")
                .arg(filter)
                .output()
                .await?
        };
        let mut results = vec![];
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(s) = ScanResult::try_from(line) {
                results.push(s);
            }
        }
        Ok(results)
    }

    pub async fn connect(&mut self, device: &Device) -> Result<()> {
        if !device.paired {
            self.pair(device.addr).await?;
        }
        if !device.trusted {
            self.trust(device.addr).await?;
        }
        let output = Command::new("bluetoothctl")
            .arg("connect")
            .arg(device.addr.to_string())
            .output()
            .await?;
        self.log_channel
            .send(String::from_utf8_lossy(&output.stdout).to_string())
            .await?;

        Ok(())
    }

    async fn trust(&mut self, addr: MacAddr6) -> Result<()> {
        let output = Command::new("bluetoothctl")
            .arg("trust")
            .arg(addr.to_string())
            .output()
            .await?;
        self.log_channel
            .send(String::from_utf8_lossy(&output.stdout).to_string())
            .await?;
        Ok(())
    }

    async fn pair(&mut self, addr: MacAddr6) -> Result<()> {
        let output = Command::new("bluetoothctl")
            .arg("pair")
            .arg(addr.to_string())
            .output()
            .await?;
        self.log_channel
            .send(String::from_utf8_lossy(&output.stdout).to_string())
            .await?;
        Ok(())
    }

    pub async fn process_requests(&mut self) -> Result<()> {
        while !self.request_channel.is_empty() {
            if let Ok(request) = self.request_channel.try_recv() {
                match request {
                    // TODO: Why dont we get here?
                    BluetoothRequest::Connect(device) => {
                        self.log_channel
                            .send(format!("Connecting to {}", device.name))
                            .await?;
                        self.connect(&device).await?;
                    }
                }
            }
        }

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
