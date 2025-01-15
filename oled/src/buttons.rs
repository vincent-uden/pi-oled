use anyhow::Result;
use rppal::gpio::{Gpio, InputPin};

#[derive(Debug)]
pub enum Button {
    B1,
    B2,
    B3,
}

#[derive(Debug)]
pub struct Buttons {
    b1: InputPin,
    b2: InputPin,
    b3: InputPin,

    b1_pressed: bool,
    b2_pressed: bool,
    b3_pressed: bool,

    b1_just_pressed: bool,
    b2_just_pressed: bool,
    b3_just_pressed: bool,
}

impl Buttons {
    pub fn pi_zero_2_w() -> Result<Self> {
        let gpio = Gpio::new()?;

        let b1 = gpio.get(21).unwrap().into_input_pullup();
        let b2 = gpio.get(20).unwrap().into_input_pullup();
        let b3 = gpio.get(16).unwrap().into_input_pullup();

        Ok(Self {
            b1,
            b2,
            b3,
            b1_just_pressed: false,
            b2_just_pressed: false,
            b3_just_pressed: false,
            b1_pressed: false,
            b2_pressed: false,
            b3_pressed: false,
        })
    }

    pub fn update(&mut self) -> Result<()> {
        if self.b1.is_low() {
            if !self.b1_pressed {
                self.b1_just_pressed = true;
            } else {
                self.b1_just_pressed = false;
            }
            self.b1_pressed = true;
        } else {
            self.b1_pressed = false;
            self.b1_just_pressed = false;
        }

        if self.b2.is_low() {
            if !self.b2_pressed {
                self.b2_just_pressed = true;
            } else {
                self.b2_just_pressed = false;
            }
            self.b2_pressed = true;
        } else {
            self.b2_pressed = false;
            self.b2_just_pressed = false;
        }

        if self.b3.is_low() {
            if !self.b3_pressed {
                self.b3_just_pressed = true;
            } else {
                self.b3_just_pressed = false;
            }
            self.b3_pressed = true;
        } else {
            self.b3_pressed = false;
            self.b3_just_pressed = false;
        }
        Ok(())
    }

    // TODO: Use the raylib names here
    pub fn is_button_pressed(&self, button: Button) -> bool {
        match button {
            Button::B1 => self.b1_just_pressed,
            Button::B2 => self.b2_just_pressed,
            Button::B3 => self.b3_just_pressed,
        }
    }

    pub fn is_button_held(&self, button: Button) -> bool {
        match button {
            Button::B1 => self.b1_pressed,
            Button::B2 => self.b2_pressed,
            Button::B3 => self.b3_pressed,
        }
    }
}
