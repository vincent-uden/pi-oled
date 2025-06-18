#!/bin/bash

# Quick setup for a blank raspberri pi

# Build dependencies
sudo apt -y install git build-essential

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal
. "$HOME/.cargo/env"

# Runtime dependencies
sudo apt -y install bluez bluez-firmware vlc

# Project setup
cd ~
mkdir github
cd github
git clone https://github.com/vincent-uden/pi-oled
cd pi-oled
cargo r --bin remote-dev -- server --port 3000

# TODO: Figure out how to programatically enable SPI using sudo raspi-config
