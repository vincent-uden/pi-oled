use std::{
    collections::HashSet,
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

use bitmap_font::{tamzen::FONT_5x9, TextStyle};
use bluetooth::{BluetoothEvent, BluetoothManager, BluetoothRequest, Device};
use buttons::{Button, Buttons};
use display::Display;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, text::Text};
use joystick::Joystick;
use local_ip_address::local_ip;

use dotenv::dotenv;
use macaddr::MacAddr6;
use tokio::process::Command;

// TODO: Manage to connect with just bluetoothctl. As per http://www.adammil.net/blog/v137_Creating_a_Bluetooth_music_player_with_a_Raspberry_Pi_Zero_2_W.html
// the bluetooth and wifi are probably messing with each other. Can I finish the program
// regardless?
//
// TODO: Set the default sink after connecting to the device

#[derive(Debug)]
pub enum Tab {
    Files,
    Network,
    Bluetooth,
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
        })
    }

    pub fn draw(&mut self) {
        match self.open_tab {
            Tab::Files => self.draw_files_tab(),
            Tab::Network => self.draw_network_tab(),
            Tab::Bluetooth => self.draw_bluetooth_tab(),
        }

        let label = match self.open_tab {
            Tab::Files => "Files",
            Tab::Network => "Network",
            Tab::Bluetooth => "Bluetooth",
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
                let file_name = &file.file_name().unwrap().to_str().unwrap()[0..self.max_len];
                let text = Text::new(
                    file_name,
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
                let text = Text::new(
                    &device.name,
                    Point::new(0, 10 + (i as i32 * self.font_height)),
                    TextStyle::new(&FONT_5x9, text_color),
                );
                text.draw(&mut self.display).unwrap();
            }
        }
    }

    pub async fn update(&mut self) -> Result<()> {
        self.buttons.update().unwrap();
        self.joystick.update().unwrap();
        if self.buttons.is_button_pressed(Button::B3) {
            self.running = false;
            return Ok(());
        }
        match self.open_tab {
            Tab::Files => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Bluetooth;
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
            }
            Tab::Network => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Files;
                }
                if self.joystick.just_switched_to(joystick::State::Right) {
                    self.open_tab = Tab::Bluetooth;
                }
            }
            Tab::Bluetooth => {
                if self.joystick.just_switched_to(joystick::State::Left) {
                    self.open_tab = Tab::Network;
                }
                if self.joystick.just_switched_to(joystick::State::Right) {
                    self.open_tab = Tab::Files;
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
        println!("BT Event");
        match event {
            BluetoothEvent::Scan(results) => {
                for result in results {
                    if !self.has_discovered_device(result.addr) && !result.name.is_empty() {
                        self.devices.push(Device::from(result));
                    }
                }
            }
        }
    }

    fn has_discovered_device(&self, addr: MacAddr6) -> bool {
        self.devices.iter().any(|d| d.addr == addr)
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let audio_dir =
        std::env::var("AUDIO_DIR").expect("The AUDIO_DIR environment variable has to be set");

    let (bt_tx, mut bt_rx) = tokio::sync::mpsc::channel::<BluetoothRequest>(10);
    let mut state = State::new(audio_dir, bt_tx).unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BluetoothEvent>(10);
    let (tx2, mut rx2) = tokio::sync::mpsc::channel::<String>(10);

    let bluetooth_task = tokio::spawn(async move {
        let mut bluetooth_manager = BluetoothManager::new(tx, tx2, bt_rx).await.unwrap();
        bluetooth_manager.start_scan().await?;

        loop {
            bluetooth_manager.process_requests().await?;
            bluetooth_manager.get_devices().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    while state.running {
        while let Ok(event) = rx2.try_recv() {
            //println!("Event: {:#?}", event);
        }

        while let Ok(event) = rx.try_recv() {
            state.handle_bluetooth_event(event);
        }
        // TODO: Move to a diff-based rendering to avoid unnecessary pixel updates
        state.display.fill(BinaryColor::Off);
        state.update().await?;

        state.draw();
        state.display.render().unwrap();

        sleep(Duration::from_millis(50));
    }

    bluetooth_task.abort();

    println!("Device initialized!");
    Ok(())
}
