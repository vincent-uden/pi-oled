use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub enum MpvEvent {
    Error(String),
    StatusUpdate {
        is_playing: bool,
        position: u32,
        duration: u32,
        filename: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum MpvRequest {
    Play(PathBuf),
    GetStatus,
}

pub struct MpvManager {
    event_channel: tokio::sync::mpsc::Sender<MpvEvent>,
    request_channel: tokio::sync::mpsc::Receiver<MpvRequest>,
    mpv_process: Option<Child>,
    socket_path: String,
}

impl MpvManager {
    pub async fn new(
        event_channel: tokio::sync::mpsc::Sender<MpvEvent>,
        request_channel: tokio::sync::mpsc::Receiver<MpvRequest>,
    ) -> Result<Self> {
        Ok(Self {
            event_channel,
            request_channel,
            mpv_process: None,
            socket_path: "/tmp/mpvsocket".to_string(),
        })
    }

    pub async fn process_requests(&mut self) -> Result<()> {
        while let Ok(request) = self.request_channel.try_recv() {
            info!("Processing MPV request: {:?}", request);
            match request {
                MpvRequest::Play(path_buf) => {
                    self.toggle_play_pause(&path_buf).await?;
                }
                MpvRequest::GetStatus => {
                    self.get_status().await?;
                }
            };
        }
        Ok(())
    }

    async fn play(&mut self, path: &Path) -> Result<()> {
        debug!("Playing a new file {:?}", path);
        match Command::new("mpv")
            .arg("--audio-buffer=0.5")
            .arg("--input-ipc-server=/tmp/mpvsocket")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                self.mpv_process = Some(child);
            }
            Err(e) => {
                error!("Starting the MPV process failed with error {}", e);
            }
        };

        Ok(())
    }

    async fn toggle_play_pause(&mut self, path: &Path) -> Result<()> {
        match &self.mpv_process {
            Some(_) => {
                debug!("A process exists, trying to pause");
                let output = Command::new("bash")
                    .arg("-c")
                    .arg("echo '{ \"command\": [\"cycle\", \"pause\"] }' | socat - /tmp/mpvsocket")
                    .output()
                    .await?;
                debug!("{}", String::from_utf8_lossy(&output.stdout).to_string());
            }
            None => {
                self.play(path).await?;
            }
        };
        Ok(())
    }

    async fn send_command(&self, command: &str) -> Result<String> {
        debug!("Sending MPV command: {}", command);

        let bash_command = format!("echo '{}' | socat - {}", command, self.socket_path);
        debug!("Full bash command: {}", bash_command);

        let output = Command::new("bash")
            .arg("-c")
            .arg(&bash_command)
            .output()
            .await?;

        if output.status.success() {
            let response = String::from_utf8_lossy(&output.stdout).to_string();
            debug!("MPV command successful, response: {}", response.trim());
            Ok(response)
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("MPV command failed: {}", error_msg);
            Err(anyhow!("MPV command failed: {}", error_msg))
        }
    }

    async fn get_status(&self) -> Result<()> {
        if self.mpv_process.is_none() {
            return Ok(());
        }

        let mut is_playing = false;
        let mut position = 0u32;
        let mut duration = 0u32;
        let mut filename: Option<String> = None;

        if let Ok(pause_response) = self.send_command(r#"{ "command": ["get_property", "pause"] }"#).await {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&pause_response) {
                if let Some(data) = parsed.get("data") {
                    is_playing = !data.as_bool().unwrap_or(true);
                }
            }
        }

        if let Ok(pos_response) = self.send_command(r#"{ "command": ["get_property", "time-pos"] }"#).await {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&pos_response) {
                if let Some(data) = parsed.get("data") {
                    position = data.as_f64().unwrap_or(0.0) as u32;
                }
            }
        }

        if let Ok(dur_response) = self.send_command(r#"{ "command": ["get_property", "duration"] }"#).await {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&dur_response) {
                if let Some(data) = parsed.get("data") {
                    duration = data.as_f64().unwrap_or(0.0) as u32;
                }
            }
        }

        if let Ok(file_response) = self.send_command(r#"{ "command": ["get_property", "filename"] }"#).await {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&file_response) {
                if let Some(data) = parsed.get("data") {
                    filename = data.as_str().map(|s| s.to_string());
                }
            }
        }

        let event = MpvEvent::StatusUpdate {
            is_playing,
            position,
            duration,
            filename,
        };

        if let Err(e) = self.event_channel.try_send(event) {
            debug!("Failed to send status update: {}", e);
        }

        Ok(())
    }
}

impl Drop for MpvManager {
    fn drop(&mut self) {
        if let Some(mut process) = self.mpv_process.take() {
            let _ = process.kill();
        }
        // Clean up socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
