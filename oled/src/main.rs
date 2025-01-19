use std::{
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

use bitmap_font::{
    tamzen::{FONT_5x9, FONT_8x15},
    TextStyle,
};
use bluetooth::{BluetoothEvent, BluetoothManager};
use buttons::{Button, Buttons};
use display::Display;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, text::Text};
use joystick::Joystick;
use local_ip_address::local_ip;

use dotenv::dotenv;
use tokio::sync::Mutex;

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
    pub fn new(audio_dir: String) -> Result<Self> {
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

    fn draw_bluetooth_tab(&mut self) {}

    pub fn update(&mut self) {
        self.buttons.update().unwrap();
        self.joystick.update().unwrap();
        if self.buttons.is_button_pressed(Button::B3) {
            self.running = false;
            return;
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
            }
        }
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let audio_dir =
        std::env::var("AUDIO_DIR").expect("The AUDIO_DIR environment variable has to be set");

    let mut device = State::new(audio_dir).unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BluetoothEvent>(10);
    let (tx2, mut rx2) = tokio::sync::mpsc::channel::<String>(10);

    let bluetooth_task = tokio::spawn(async move {
        let mut bluetooth_manager = BluetoothManager::new(tx, tx2).await.unwrap();
        bluetooth_manager.start_scan().await?;

        loop {
            bluetooth_manager.get_devices().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    });

    while device.running {
        while let Ok(event) = rx2.try_recv() {
            println!("Event: {:#?}", event);
        }

        while let Ok(event) = rx.try_recv() {
            match event {
                BluetoothEvent::Scan(devices) => {
                    println!("Scanning for devices: {:#?}", devices);
                    for device in devices {
                        println!("Device: {:#?}", device);
                    }
                }
            }
        }
        // TODO: Move to a diff-based rendering to avoid unnecessary pixel updates
        device.display.fill(BinaryColor::Off);
        device.update();

        device.draw();
        device.display.render().unwrap();

        sleep(Duration::from_millis(50));
    }

    bluetooth_task.abort();

    println!("Device initialized!");
    Ok(())
}
