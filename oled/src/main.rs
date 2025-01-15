use std::{thread::sleep, time::Duration};

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

pub struct Device {
    pub display: Display,
    pub joystick: Joystick,
    pub buttons: Buttons,
}

impl Device {
    pub fn new() -> Result<Self> {
        Ok(Self {
            display: Display::pi_zero_2_w(128, 64)?,
            joystick: Joystick::pi_zero_2_w()?,
            buttons: Buttons::pi_zero_2_w()?,
        })
    }
}

fn main() {
    let mut device = Device::new().unwrap();

    let text = Text::new(
        "Hi",
        Point::new(10, 10),
        TextStyle::new(&FONT_5x9, BinaryColor::On),
    );

    let mut running = true;
    while running {
        device.buttons.update().unwrap();

        if device.buttons.is_button_pressed(Button::B3) {
            running = false;
        }

        text.draw(&mut device.display).unwrap();

        device.display.render().unwrap();
        sleep(Duration::from_millis(50));
    }

    println!("Device initialized!");
}
