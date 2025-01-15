use std::{thread::sleep, time::Duration};

use anyhow::Result;

mod buttons;
mod display;
mod joystick;

use buttons::{Button, Buttons};
use display::Display;
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
    device.display.render().unwrap();

    let mut running = true;
    while running {
        device.buttons.update().unwrap();

        if device.buttons.is_button_pressed(Button::B3) {
            running = false;
        }
        sleep(Duration::from_millis(100));
    }

    println!("Device initialized!");
}
