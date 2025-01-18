use std::{net::IpAddr, thread::sleep, time::Duration};

use anyhow::Result;

mod buttons;
mod display;
mod joystick;

use bitmap_font::{
    tamzen::{FONT_5x9, FONT_8x15},
    TextStyle,
};
use buttons::{Button, Buttons};
use display::Display;
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, text::Text};
use joystick::Joystick;
use local_ip_address::local_ip;

#[derive(Debug)]
pub enum Tab {
    Files,
    Network,
    Bluetooth,
}

pub struct Device {
    pub display: Display,
    pub joystick: Joystick,
    pub buttons: Buttons,
    open_tab: Tab,
    ip: IpAddr,
}

impl Device {
    pub fn new() -> Result<Self> {
        Ok(Self {
            display: Display::pi_zero_2_w(128, 64)?,
            joystick: Joystick::pi_zero_2_w()?,
            buttons: Buttons::pi_zero_2_w()?,
            open_tab: Tab::Files,
            ip: local_ip()?,
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

    fn draw_files_tab(&mut self) {}

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
}

fn main() {
    let mut device = Device::new().unwrap();

    let mut running = true;
    while running {
        device.buttons.update().unwrap();
        device.joystick.update().unwrap();

        if device.buttons.is_button_pressed(Button::B3) {
            running = false;
        }
        if device.joystick.just_switched_to(joystick::State::Left) {
            device.open_tab = match device.open_tab {
                Tab::Files => Tab::Network,
                Tab::Network => Tab::Bluetooth,
                Tab::Bluetooth => Tab::Files,
            };
        }
        if device.joystick.just_switched_to(joystick::State::Right) {
            device.open_tab = match device.open_tab {
                Tab::Files => Tab::Bluetooth,
                Tab::Network => Tab::Files,
                Tab::Bluetooth => Tab::Network,
            };
        }

        device.display.fill(BinaryColor::Off);

        device.draw();
        device.display.render().unwrap();

        sleep(Duration::from_millis(50));
    }

    println!("Device initialized!");
}
