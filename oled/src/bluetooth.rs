use std::{hash::Hash, process::Stdio};

use anyhow::Result;
use macaddr::MacAddr6;
use tokio::process::{Child, Command};
use tracing::{debug, info};

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
            // TODO: Populate these
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

#[derive(Debug, Clone)]
pub enum BluetoothEvent {
    Scan(Vec<Device>),
}

#[derive(Debug, Clone)]
pub enum BluetoothRequest {
    Connect(Device),
    Disconnect(Device),
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
        debug!("Before scan");
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
        debug!("After scan");
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
        let all = self.devices().await?;
        let mut devices: Vec<Device> = all.into_iter().map(|x| x.into()).collect();
        for d in devices.iter_mut() {
            let output = Command::new("bluetoothctl")
                .arg("info")
                .arg(format!("{}", d.addr))
                .output()
                .await?;
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if line.trim_start().starts_with("Paired:") {
                    let status = line.split(": ").skip(1).next();
                    if status == Some("yes") {
                        d.paired = true;
                    }
                    if status == Some("no") {
                        d.paired = false;
                    }
                }
                if line.trim_start().starts_with("Trusted:") {
                    let status = line.split(": ").skip(1).next();
                    if status == Some("yes") {
                        d.trusted = true;
                    }
                    if status == Some("no") {
                        d.trusted = false;
                    }
                }
                if line.trim_start().starts_with("Connected:") {
                    let status = line.split(": ").skip(1).next();
                    if status == Some("yes") {
                        d.connected = true;
                    }
                    if status == Some("no") {
                        d.connected = false;
                    }
                }
            }
        }

        self.channel.send(BluetoothEvent::Scan(devices)).await?;

        Ok(())
    }

    /// Assumes that scanning is running in the background
    async fn devices(&mut self) -> Result<Vec<ScanResult>> {
        let output = Command::new("bluetoothctl").arg("devices").output().await?;
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
        info!("Inside connecting");
        if device.connected {
            let output = Command::new("bluetoothctl")
                .arg("disconnect")
                .arg(device.addr.to_string())
                .output()
                .await?;
            debug!("{}", String::from_utf8_lossy(&output.stdout).to_string());
            info!("Disconnecting from {:?}", device);
        } else {
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
            debug!("{}", String::from_utf8_lossy(&output.stdout).to_string());
            info!("Connecting to {:?}", device);
        }

        Ok(())
    }

    async fn trust(&mut self, addr: MacAddr6) -> Result<()> {
        let output = Command::new("bluetoothctl")
            .arg("trust")
            .arg(addr.to_string())
            .output()
            .await?;
        debug!("{}", String::from_utf8_lossy(&output.stdout).to_string());
        Ok(())
    }

    async fn pair(&mut self, addr: MacAddr6) -> Result<()> {
        let output = Command::new("bluetoothctl")
            .arg("pair")
            .arg(addr.to_string())
            .output()
            .await?;
        debug!("{}", String::from_utf8_lossy(&output.stdout).to_string());
        Ok(())
    }

    pub async fn process_requests(&mut self) -> Result<()> {
        // This is working!
        while !self.request_channel.is_empty() {
            if let Ok(request) = self.request_channel.try_recv() {
                info!("Processing {:?}", request);
                match request {
                    BluetoothRequest::Connect(device) => {
                        self.connect(&device).await?;
                        self.stop_scan().await?;
                    }
                    BluetoothRequest::Disconnect(device) => {
                        self.disconnect(&device).await?;
                        self.start_scan().await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn disconnect(&self, device: &Device) -> _ {
        todo!()
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
