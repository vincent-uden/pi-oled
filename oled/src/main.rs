use std::{
    collections::HashSet,
    io,
    net::IpAddr,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use anyhow::{anyhow, Result};

mod bluetooth;
mod buttons;
mod display;
mod joystick;
mod mpv;

use bitmap_font::{tamzen::FONT_5x9, TextStyle};
use bluetooth::{BluetoothEvent, BluetoothManager, BluetoothRequest, Device};
use buttons::{Button, Buttons};
use display::Display;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, text::Text};
use joystick::Joystick;
use local_ip_address::local_ip;
use mpv::{MpvEvent, MpvManager, MpvRequest};

use dotenv::dotenv;
use macaddr::MacAddr6;
use tokio::process::Command;
use tracing::{debug, error, info, warn, Level};
use tracing_log::log::LevelFilter;
use tracing_subscriber::EnvFilter;

// TODO: Set the default sink after connecting to the device

#[derive(Debug)]
pub enum Tab {
    Files,
    Network,
    Bluetooth,
    Player,
}

pub struct State {
    pub display: Display,
    pub joystick: Joystick,
    pub buttons: Buttons,
    pub devices: Vec<Device>,
    open_tab: Tab,
    ip: IpAddr,
    audio_files: Vec<PathBuf>,
    audio_dir: PathBuf,
    font_width: i32,
    font_height: i32,
    file_scroll: i32,
    file_cursor: i32,
    running: bool,
    max_files: i32,
    max_len: usize,
    bt_scroll: i32,
    bt_cursor: i32,
    bt_channel: tokio::sync::mpsc::Sender<BluetoothRequest>,
    mpv_channel: tokio::sync::mpsc::Sender<MpvRequest>,
    player_status: PlayerStatus,
    system_volume: u8,
    track_position: u32,
    track_duration: u32,
    filename_scroll_offset: usize,
    filename_scroll_counter: u32,
    wifi_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerStatus {
    pub is_playing: bool,
    pub current_file: Option<String>,
}

fn files_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    files
}

impl State {
    pub fn new(
        audio_dir: String,
        bt_channel: tokio::sync::mpsc::Sender<BluetoothRequest>,
        mpv_channel: tokio::sync::mpsc::Sender<MpvRequest>,
    ) -> Result<Self> {
        let audio_dir = PathBuf::from(audio_dir);
        if !audio_dir.exists() {
            return Err(anyhow!("Audio directory does not exist"));
        }
        if !audio_dir.is_dir() {
            return Err(anyhow!("Audio directory is not a directory"));
        }
        let available_height = 64 - 10;
        let font_width = 5;
        let font_height = 9;
        let max_files = available_height / font_height;
        let max_len = 128 / font_width as usize;
        Ok(Self {
            display: Display::pi_zero_2_w(128, 64)?,
            joystick: Joystick::pi_zero_2_w()?,
            buttons: Buttons::pi_zero_2_w()?,
            open_tab: Tab::Files,
            devices: Vec::new(),
            ip: local_ip()?,
            audio_files: files_in_dir(&audio_dir),
            audio_dir,
            font_width,
            font_height,
            file_scroll: 0,
            file_cursor: 0,
            running: true,
            max_files,
            max_len,
            bt_scroll: 0,
            bt_cursor: 0,
            bt_channel,
            mpv_channel,
            player_status: PlayerStatus {
                is_playing: false,
                current_file: None,
            },
            system_volume: 50,
            track_position: 0,
            track_duration: 0,
            filename_scroll_offset: 0,
            filename_scroll_counter: 0,
            wifi_enabled: true,
        })
    }

    pub fn draw(&mut self) {
        match self.open_tab {
            Tab::Files => self.draw_files_tab(),
            Tab::Network => self.draw_network_tab(),
            Tab::Bluetooth => self.draw_bluetooth_tab(),
            Tab::Player => self.draw_player_tab(),
        }

        let label = match self.open_tab {
            Tab::Files => "Files",
            Tab::Network => "Network",
            Tab::Bluetooth => "Bluetooth",
            Tab::Player => "Player",
        };
        // Centered text
        let tab_text = Text::new(
            label,
            Point::new(
                (self.display.width() - (label.len() as i32 * 5)) / 2 as i32,
                0,
            ),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        let left_arrow = Text::new(
            "<",
            Point::new(0, 0),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        let right_arrow = Text::new(
            ">",
            Point::new(self.display.width() - 5, 0),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );

        left_arrow.draw(&mut self.display).unwrap();
        right_arrow.draw(&mut self.display).unwrap();
        tab_text.draw(&mut self.display).unwrap();
    }

    fn draw_files_tab(&mut self) {
        for (i, file) in self.audio_files.iter().enumerate() {
            if (i as i32) >= self.file_scroll && (i as i32) < self.file_scroll + self.max_files {
                let text_color = if self.file_cursor == i as i32 {
                    BinaryColor::Off
                } else {
                    BinaryColor::On
                };
                if self.file_cursor == i as i32 {
                    self.display.draw_rect(
                        0,
                        (10 + (i as i32 - self.file_scroll) * self.font_height) as u8,
                        self.display.width() as u8,
                        self.font_height as u8,
                        BinaryColor::On,
                    );
                }
                let file_name = file.file_name().unwrap().to_str().unwrap();
                let clipped = if file_name.len() > self.max_len {
                    &file_name[0..self.max_len]
                } else {
                    file_name
                };
                let text = Text::new(
                    clipped,
                    Point::new(0, 10 + (i as i32 - self.file_scroll) * self.font_height),
                    TextStyle::new(&FONT_5x9, text_color),
                );
                text.draw(&mut self.display).unwrap();
            }
        }
    }

    fn draw_network_tab(&mut self) {
        let label = format!("IP: {}", self.ip);
        let ip_text = Text::new(
            &label,
            Point::new(0, 10),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        ip_text.draw(&mut self.display).unwrap();

        let wifi_status = if self.wifi_enabled { "ON" } else { "OFF" };
        let wifi_label = format!("WiFi: {}", wifi_status);
        let wifi_text = Text::new(
            &wifi_label,
            Point::new(0, 20),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        wifi_text.draw(&mut self.display).unwrap();

        let instruction_text = Text::new(
            "B1: Toggle WiFi",
            Point::new(0, 40),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        instruction_text.draw(&mut self.display).unwrap();
    }

    fn draw_bluetooth_tab(&mut self) {
        for (i, device) in self.devices.iter().enumerate() {
            if (i as i32) >= self.bt_scroll && (i as i32) < self.bt_scroll + self.max_files {
                let text_color = if self.bt_cursor == i as i32 {
                    BinaryColor::Off
                } else {
                    BinaryColor::On
                };
                if self.bt_cursor == i as i32 {
                    self.display.draw_rect(
                        0,
                        (10 + (i as i32 - self.bt_scroll) * self.font_height) as u8,
                        self.display.width() as u8,
                        self.font_height as u8,
                        BinaryColor::On,
                    );
                }
                let mut label: Vec<char> = device.name.chars().take(self.max_len).collect();
                let label_len = label.len();
                for _ in 0..(self.max_len - label_len) {
                    label.push(' ');
                }
                label[self.max_len - 3] = if device.connected { 'o' } else { 'x' };
                label[self.max_len - 2] = if device.trusted { 'o' } else { 'x' };
                label[self.max_len - 1] = if device.paired { 'o' } else { 'x' };
                let label: String = label.into_iter().collect();

                let text = Text::new(
                    &label,
                    Point::new(0, 10 + (i as i32 * self.font_height)),
                    TextStyle::new(&FONT_5x9, text_color),
                );
                text.draw(&mut self.display).unwrap();
            }
        }
    }

    fn draw_player_tab(&mut self) {
        let status = if self.player_status.is_playing {
            "Playing"
        } else {
            "Paused"
        };
        let status_text = Text::new(
            status,
            Point::new(0, 10),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        status_text.draw(&mut self.display).unwrap();

        let volume_label = format!("Vol: {}%", self.system_volume);
        let volume_text = Text::new(
            &volume_label,
            Point::new(0, 20),
            TextStyle::new(&FONT_5x9, BinaryColor::On),
        );
        volume_text.draw(&mut self.display).unwrap();

        if self.track_duration > 0 {
            let pos_min = self.track_position / 60;
            let pos_sec = self.track_position % 60;
            let dur_min = self.track_duration / 60;
            let dur_sec = self.track_duration % 60;
            let progress_label = format!("{}:{:02} / {}:{:02}", pos_min, pos_sec, dur_min, dur_sec);
            let progress_text = Text::new(
                &progress_label,
                Point::new(0, 30),
                TextStyle::new(&FONT_5x9, BinaryColor::On),
            );
            progress_text.draw(&mut self.display).unwrap();
        }

        if let Some(ref filename) = self.player_status.current_file {
            let display_name = if filename.len() > self.max_len {
                self.get_scrolling_text(filename)
            } else {
                filename.clone()
            };
            let file_text = Text::new(
                &display_name,
                Point::new(0, 40),
                TextStyle::new(&FONT_5x9, BinaryColor::On),
            );
            file_text.draw(&mut self.display).unwrap();
        }
    }

    pub async fn update(&mut self) -> Result<()> {
        self.buttons.update().unwrap();
        self.joystick.update().unwrap();
        if self.buttons.is_button_pressed(Button::B3) {
            self.running = false;
            return Ok(());
        }

        if let Ok(volume) = self.get_system_volume().await {
            self.system_volume = volume;
        }

        if let Ok(wifi_status) = self.get_wifi_status().await {
            self.wifi_enabled = wifi_status;
        }

        self.update_filename_scroll();
        match self.open_tab {
            Tab::Files => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Player;
                }
                if self.joystick.just_switched_to(joystick::State::Right) {
                    self.open_tab = Tab::Network;
                }

                if self.joystick.just_switched_to(joystick::State::Up) {
                    self.move_file_cursor(-1);
                }
                if self.joystick.just_switched_to(joystick::State::Down) {
                    self.move_file_cursor(1);
                }
                if self.buttons.is_button_pressed(Button::B1) {
                    if let Some(file) = self.audio_files.get(self.file_cursor as usize) {
                        info!("B1 pressed - loading file: {:?}", file);
                        if let Err(e) = self.bt_channel.try_send(BluetoothRequest::StopScan) {
                            error!("Failed to send StopScan request: {}", e);
                        }
                        if let Err(e) = self.mpv_channel.try_send(MpvRequest::Play(file.clone())) {
                            error!("Failed to send LoadFile request: {}", e);
                        }
                    } else {
                        warn!(
                            "B1 pressed but no file selected (cursor: {})",
                            self.file_cursor
                        );
                    }
                }
            }
            Tab::Network => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Files;
                }
                if self.joystick.just_switched_to(joystick::State::Right) {
                    self.open_tab = Tab::Bluetooth;
                }
                if self.buttons.is_button_pressed(Button::B1) {
                    if let Err(e) = self.toggle_wifi().await {
                        error!("Failed to toggle WiFi: {}", e);
                    }
                }
            }
            Tab::Bluetooth => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Network;
                }
                if self.joystick.just_switched_to(joystick::State::Right) {
                    self.open_tab = Tab::Player;
                }
                if self.joystick.just_switched_to(joystick::State::Up) {
                    self.move_bt_cursor(-1);
                }
                if self.joystick.just_switched_to(joystick::State::Down) {
                    self.move_bt_cursor(1);
                }
                if self.buttons.is_button_pressed(Button::B1) {
                    let device = &self.devices[self.bt_cursor as usize];
                    println!("Sending Connecting to {}", device.name);
                    self.bt_channel
                        .send(BluetoothRequest::Connect(device.clone()))
                        .await?;
                }
            }
            Tab::Player => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Bluetooth;
                }
                if self.joystick.just_switched_to(joystick::State::Right) {
                    self.open_tab = Tab::Files;
                }
                if self.joystick.just_switched_to(joystick::State::Up) {
                    if let Err(e) = self.volume_up().await {
                        error!("Failed to increase volume: {}", e);
                    }
                }
                if self.joystick.just_switched_to(joystick::State::Down) {
                    if let Err(e) = self.volume_down().await {
                        error!("Failed to decrease volume: {}", e);
                    }
                }
                if self.buttons.is_button_pressed(Button::B1) {
                    if let Err(e) = self.mpv_channel.try_send(MpvRequest::TogglePause) {
                        error!("Failed to send TogglePause request: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    fn move_file_cursor(&mut self, direction: i32) {
        self.file_cursor += direction;
        if self.file_cursor < 0 {
            self.file_cursor = 0;
        }
        if self.file_cursor >= self.audio_files.len() as i32 {
            self.file_cursor = self.audio_files.len() as i32 - 1;
        }
        if self.file_cursor + self.file_scroll >= self.max_files {
            self.file_scroll += 1;
            if self.file_scroll >= self.audio_files.len() as i32 - self.max_files {
                self.file_scroll = self.audio_files.len() as i32 - self.max_files;
            }
        } else if self.file_cursor <= self.file_scroll {
            self.file_scroll -= 1;
            if self.file_scroll < 0 {
                self.file_scroll = 0;
            }
        }
    }

    fn move_bt_cursor(&mut self, direction: i32) {
        self.bt_cursor += direction;
        if self.bt_cursor < 0 {
            self.bt_cursor = 0;
        }
        if self.bt_cursor >= self.devices.len() as i32 {
            self.bt_cursor = self.devices.len() as i32 - 1;
        }
        if self.bt_cursor + self.bt_scroll >= self.max_files {
            self.bt_scroll += 1;
            if self.bt_scroll >= self.devices.len() as i32 - self.max_files {
                self.bt_scroll = self.devices.len() as i32 - self.max_files;
            }
        } else if self.bt_cursor <= self.bt_scroll {
            self.bt_scroll -= 1;
            if self.bt_scroll < 0 {
                self.bt_scroll = 0;
            }
        }
        println!("Scroll: {} Cursor: {}", self.bt_scroll, self.bt_cursor)
    }

    pub fn handle_bluetooth_event(&mut self, event: BluetoothEvent) {
        match event {
            BluetoothEvent::Scan(results) => {
                self.devices = results.into_iter().filter(|d| !d.name.is_empty()).collect();
            }
        }
    }

    pub fn handle_mpv_event(&mut self, event: MpvEvent) {
        debug!("Handling MPV event: {:?}", event);
        match event {
            MpvEvent::Error(err) => {
                error!("MPV Error: {}", err);
            }
            MpvEvent::StatusUpdate {
                is_playing,
                position,
                duration,
                filename,
            } => {
                if self.player_status.current_file != filename {
                    self.filename_scroll_offset = 0;
                    self.filename_scroll_counter = 0;
                }
                self.player_status.is_playing = is_playing;
                self.player_status.current_file = filename;
                self.track_position = position;
                self.track_duration = duration;
            }
        }
    }

    async fn get_system_volume(&mut self) -> Result<u8> {
        let output = Command::new("pactl")
            .arg("get-sink-volume")
            .arg("@DEFAULT_SINK@")
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(volume_line) = output_str.lines().next() {
            if let Some(percent_pos) = volume_line.find('%') {
                let start = volume_line[..percent_pos].rfind(' ').unwrap_or(0) + 1;
                if let Ok(volume) = volume_line[start..percent_pos].parse::<u8>() {
                    return Ok(volume);
                }
            }
        }
        Ok(50)
    }

    fn get_scrolling_text(&self, text: &str) -> String {
        if text.len() <= self.max_len {
            return text.to_string();
        }

        let extended_text = format!("{} --- ", text);
        let total_len = extended_text.len();

        if self.filename_scroll_offset >= total_len {
            return extended_text[0..self.max_len].to_string();
        }

        let end_pos = (self.filename_scroll_offset + self.max_len).min(total_len);
        let mut result = extended_text[self.filename_scroll_offset..end_pos].to_string();

        if result.len() < self.max_len {
            let remaining = self.max_len - result.len();
            let wrap_text = &extended_text[0..remaining.min(total_len)];
            result.push_str(wrap_text);
        }

        result
    }

    fn update_filename_scroll(&mut self) {
        if let Some(ref filename) = self.player_status.current_file {
            if filename.len() > self.max_len {
                self.filename_scroll_counter += 1;
                if self.filename_scroll_counter >= 15 {
                    self.filename_scroll_counter = 0;
                    self.filename_scroll_offset += 1;
                    let extended_len = filename.len() + 5;
                    if self.filename_scroll_offset >= extended_len {
                        self.filename_scroll_offset = 0;
                    }
                }
            } else {
                self.filename_scroll_offset = 0;
                self.filename_scroll_counter = 0;
            }
        }
    }

    async fn volume_up(&mut self) -> Result<()> {
        Command::new("pactl")
            .arg("set-sink-volume")
            .arg("@DEFAULT_SINK@")
            .arg("+5%")
            .spawn()?
            .wait()
            .await?;
        Ok(())
    }

    async fn volume_down(&mut self) -> Result<()> {
        Command::new("pactl")
            .arg("set-sink-volume")
            .arg("@DEFAULT_SINK@")
            .arg("-5%")
            .spawn()?
            .wait()
            .await?;
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        Command::new("pactl")
            .arg("suspend-sink")
            .arg("@DEFAULT_SINK@")
            .arg("1")
            .spawn()?
            .wait()
            .await?;
        Ok(())
    }

    async fn unpause(&mut self) -> Result<()> {
        Command::new("pactl")
            .arg("suspend-sink")
            .arg("@DEFAULT_SINK@")
            .arg("0")
            .spawn()?
            .wait()
            .await?;
        Ok(())
    }

    async fn get_wifi_status(&self) -> Result<bool> {
        let output = Command::new("rfkill")
            .arg("list")
            .arg("wifi")
            .output()
            .await?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("Soft blocked:") {
                return Ok(!line.contains("yes"));
            }
        }
        Ok(true)
    }

    async fn toggle_wifi(&mut self) -> Result<()> {
        let command = if self.wifi_enabled { "block" } else { "unblock" };
        Command::new("rfkill")
            .arg(command)
            .arg("wifi")
            .spawn()?
            .wait()
            .await?;
        
        self.wifi_enabled = !self.wifi_enabled;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let audio_dir =
        std::env::var("AUDIO_DIR").expect("The AUDIO_DIR environment variable has to be set");

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("oled=debug,oled::mpv=debug"))
        .with_max_level(Level::DEBUG)
        .init();
    info!("testing tracing");

    let (bt_tx, mut bt_rx) = tokio::sync::mpsc::channel::<BluetoothRequest>(10);
    let (mpv_tx, mut mpv_rx) = tokio::sync::mpsc::channel::<MpvRequest>(10);
    let mut state = State::new(audio_dir, bt_tx, mpv_tx).unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BluetoothEvent>(10);
    let (tx2, mut rx2) = tokio::sync::mpsc::channel::<String>(10);
    let (mpv_event_tx, mut mpv_event_rx) = tokio::sync::mpsc::channel::<MpvEvent>(10);

    let bluetooth_task = tokio::spawn(async move {
        debug!("BT Thread");
        let mut bluetooth_manager = BluetoothManager::new(tx, tx2, bt_rx).await.unwrap();
        // Scanning needs to be turned off when we're playing audio
        bluetooth_manager.start_scan().await?;

        loop {
            bluetooth_manager.process_requests().await?;
            bluetooth_manager.get_devices().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    let mpv_task = tokio::spawn(async move {
        info!("Starting MPV thread");

        let mut mpv_manager = match MpvManager::new(mpv_event_tx, mpv_rx).await {
            Ok(manager) => {
                info!("MPV manager created successfully");
                manager
            }
            Err(e) => {
                error!("Failed to create MPV manager: {}", e);
                return Err(e);
            }
        };

        info!("MPV manager created, entering main loop");
        loop {
            if let Err(e) = mpv_manager.process_requests().await {
                error!("Error processing MPV requests: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    debug!("Main loop");
    let mut status_counter = 0u32;
    while state.running {
        while let Ok(event) = rx2.try_recv() {
            //println!("Event: {:#?}", event);
        }

        while let Ok(event) = rx.try_recv() {
            state.handle_bluetooth_event(event);
        }

        while let Ok(event) = mpv_event_rx.try_recv() {
            state.handle_mpv_event(event);
        }

        status_counter += 1;
        if status_counter % 20 == 0 {
            if let Err(e) = state.mpv_channel.try_send(MpvRequest::GetStatus) {
                debug!("Failed to send GetStatus request: {}", e);
            }
        }

        // TODO: Move to a diff-based rendering to avoid unnecessary pixel updates
        state.display.fill(BinaryColor::Off);
        state.update().await?;

        state.draw();
        state.display.render().unwrap();

        sleep(Duration::from_millis(50));
    }

    bluetooth_task.abort();
    mpv_task.abort();

    println!("Device initialized!");
    Ok(())
}
