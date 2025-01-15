use anyhow::Result;
use rppal::gpio::{Gpio, InputPin, Level};

pub struct Joystick {
    up_pin: InputPin,
    down_pin: InputPin,
    left_pin: InputPin,
    right_pin: InputPin,
    click_pin: InputPin,
}

#[derive(Debug)]
pub enum State {
    Up,
    Down,
    Left,
    Right,
    Click,
    Neutral,
}

impl Joystick {
    pub fn pi_zero_2_w() -> Result<Self> {
        let gpio = Gpio::new()?;

        let up_pin = gpio.get(6)?.into_input_pullup();
        let down_pin = gpio.get(19)?.into_input_pullup();
        let left_pin = gpio.get(5)?.into_input_pullup();
        let right_pin = gpio.get(26)?.into_input_pullup();
        let click_pin = gpio.get(13)?.into_input_pullup();

        Ok(Self {
            up_pin,
            down_pin,
            left_pin,
            right_pin,
            click_pin,
        })
    }

    pub fn read(&mut self) -> Result<State> {
        if self.up_pin.read() == Level::Low {
            return Ok(State::Up);
        }
        if self.down_pin.read() == Level::Low {
            return Ok(State::Down);
        }
        if self.left_pin.read() == Level::Low {
            return Ok(State::Left);
        }
        if self.right_pin.read() == Level::Low {
            return Ok(State::Right);
        }
        if self.click_pin.read() == Level::Low {
            return Ok(State::Click);
        }

        Ok(State::Neutral)
    }
}
