use std::{thread::sleep, time::Duration};

use anyhow::Result;

mod display;

use display::Display;

pub struct Device {
    pub display: Display,
}

impl Device {
    pub fn new() -> Result<Self> {
        Ok(Self {
            display: Display::pi_zero_2_w(128, 64)?,
        })
    }
}

fn main() {
    let mut device = Device::new().unwrap();
    device.display.render().unwrap();

    sleep(Duration::from_millis(5000));

    println!("Device initialized!");
}
