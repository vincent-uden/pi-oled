use std::{thread::sleep, time::Duration};

use anyhow::Result;
use embedded_graphics::{
    pixelcolor::{BinaryColor, Gray2},
    prelude::{DrawTarget, GrayColor, OriginDimensions, Size},
};
use rppal::{
    gpio::{Gpio, OutputPin},
    spi::Spi,
};

const BUS_CLK_SPEED: u32 = 8_000_000;

// TODO: Turn screen off on drop
#[derive(Debug)]
pub struct Display {
    width: i32,
    height: i32,
    bus: Spi,
    rst_pin: OutputPin,
    dc_pin: OutputPin,
    cs_pin: OutputPin,
    bl_pin: OutputPin,
    buffer: Vec<u8>,
}

impl Display {
    pub fn pi_zero_2_w(width: i32, height: i32) -> Result<Self> {
        let gpio = Gpio::new()?;
        let rst_pin = gpio.get(25)?.into_output();
        let dc_pin = gpio.get(24)?.into_output();
        let cs_pin = gpio.get(8)?.into_output();
        let bl_pin = gpio.get(18)?.into_output();

        let bus = Spi::new(
            rppal::spi::Bus::Spi0,
            rppal::spi::SlaveSelect::Ss0,
            BUS_CLK_SPEED,
            rppal::spi::Mode::Mode0,
        )?;

        let mut out = Self {
            width,
            height,
            bus,
            rst_pin,
            dc_pin,
            cs_pin,
            bl_pin,
            buffer: vec![0x00; width as usize * ((height / 8) as usize)],
        };

        out.reset();
        println!("resetting");
        out.write_command(&[
            0xAE, 0x02, 0x10, 0x40, 0x81, 0xA0, 0xC0, 0xA6, 0xA8, 0x3F, 0xD3, 0x00, 0xd5, 0x80,
            0xD9, 0xF1, 0xDA, 0x12, 0xDB, 0x40, 0x20, 0x02, 0xA4, 0xA6,
        ])?;
        sleep(Duration::from_millis(100));
        out.write_command(&[0xAF])?;

        Ok(out)
    }

    fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.dc_pin.set_high();
        self.bus.write(&data)?;
        Ok(())
    }

    fn write_command(&mut self, data: &[u8]) -> Result<()> {
        self.dc_pin.set_low();
        self.bus.write(&data)?;
        Ok(())
    }

    fn reset(&mut self) {
        self.rst_pin.set_high();
        sleep(Duration::from_millis(100));
        self.rst_pin.set_low();
        sleep(Duration::from_millis(100));
        self.rst_pin.set_high();
        sleep(Duration::from_millis(100));
    }

    /// Each byte represents 8 pixels (stacked vertically) on the screen
    pub fn render(&mut self) -> Result<()> {
        for page in 0..8 {
            self.write_command(&[0xB0 + page])?;
            self.write_command(&[0x02])?;
            self.write_command(&[0x10])?;
            sleep(Duration::from_millis(10));
            self.dc_pin.set_high();
            for index in 0..(self.width as usize) {
                let byte = self.buffer[index + self.width as usize * page as usize];
                self.bus.write(&[byte])?;
            }
        }

        Ok(())
    }

    pub fn draw_pixel(&mut self, x: u8, y: u8, color: bool) {
        let index = x as usize + (y / 8) as usize * self.width as usize;
        if color {
            self.buffer[index] |= 1 << (y % 8);
        } else {
            self.buffer[index] &= !(1 << (y % 8));
        }
    }

    pub fn fill(&mut self, color: BinaryColor) {
        for byte in self.buffer.iter_mut() {
            *byte = match color {
                BinaryColor::On => 0xFF,
                BinaryColor::Off => 0x00,
            };
        }
    }

    pub fn draw_rect(&mut self, x: u8, y: u8, width: u8, height: u8, color: BinaryColor) {
        for i in 0..width {
            for j in 0..height {
                self.draw_pixel(x + i, y + j, color == BinaryColor::On);
            }
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size {
            width: self.width as u32,
            height: self.height as u32,
        }
    }
}

impl DrawTarget for Display {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> std::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for pixel in pixels {
            self.draw_pixel(pixel.0.x as u8, pixel.0.y as u8, pixel.1 == BinaryColor::On);
        }
        Ok(())
    }
}
