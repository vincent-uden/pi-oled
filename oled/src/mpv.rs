use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub enum MpvEvent {
    Error(String),
}

#[derive(Debug, Clone)]
pub enum MpvRequest {
    Play(PathBuf),
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
            };
        }
        Ok(())
    }

    async fn play(&mut self, path: &Path) -> Result<()> {
        debug!("Playing a new file {:?}", path);
        match Command::new("mpv")
            .arg("--idle")
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
