#!/bin/bash

# Quick setup for a blank raspberri pi

# Build dependencies
sudo apt -y install git build-essential

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal
. "$HOME/.cargo/env"

# Runtime dependencies
sudo apt -y install bluez bluez-firmware vlc

# Sudo permissions setup for wifi control
USERNAME="vincent" # Replace with the actual username
SUDOERS_FILE="/etc/sudoers.d/rfkill_nopasswd"
SUDOERS_RULE="${USERNAME} ALL=(ALL) NOPASSWD: /usr/sbin/rfkill"

# --- Script Logic ---

# Check if the sudoers.d directory exists
if [ ! -d "/etc/sudoers.d" ]; then
    echo "Error: /etc/sudoers.d directory does not exist. Your system might be configured differently."
    exit 1
fi

# Check if the rule already exists to prevent duplication
if grep -qF "${SUDOERS_RULE}" "${SUDOERS_FILE}" 2>/dev/null; then
    echo "Sudo rule for ${USERNAME} to use rfkill without password already exists in ${SUDOERS_FILE}."
else
    echo "Adding sudo rule for ${USERNAME} to use rfkill without password..."
    echo "${SUDOERS_RULE}" | sudo tee "${SUDOERS_FILE}" > /dev/null
    sudo chmod 0440 "${SUDOERS_FILE}"
    echo "Rule added and permissions set for ${SUDOERS_FILE}."
fi

# Project setup
cd ~
mkdir github
cd github
git clone https://github.com/vincent-uden/pi-oled
cd pi-oled
cargo r --bin remote-dev -- server --port 3000

# TODO: Figure out how to programatically enable SPI using sudo raspi-config
