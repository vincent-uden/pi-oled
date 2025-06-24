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
cargo b --bin oled --release
chmod -R 777 ./target/release

# --- Systemd Service Setup for pi-oled ---
SERVICE_NAME="pi-oled.service"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}"
OLED_EXECUTABLE="/home/${USERNAME}/github/pi-oled/target/release/oled"

echo "Creating systemd service for pi-oled..."

# Check if the executable exists
if [ ! -f "${OLED_EXECUTABLE}" ]; then
    echo "Error: OLED executable not found at ${OLED_EXECUTABLE}. Make sure 'cargo build --release' was successful."
    exit 1
fi

sudo bash -c "cat > ${SERVICE_FILE}" <<EOF
[Unit]
Description=Pi OLED Display Application
After=network.target # Assuming it might need network, adjust if not.
After=systemd-user-sessions.service # Might be useful if it interacts with user sessions

[Service]
ExecStart=${OLED_EXECUTABLE}
WorkingDirectory=/home/${USERNAME}/github/pi-oled
StandardOutput=journal
StandardError=journal
Restart=always
User=${USERNAME}
Group=${USERNAME} # Good practice to specify group too
Environment=RUST_LOG=info # Example: Set environment variables if needed by the Rust app

[Install]
WantedBy=multi-user.target
EOF

if [ -f "${SERVICE_FILE}" ]; then
    echo "Systemd service file created at ${SERVICE_FILE}."
    echo "Enabling and starting the pi-oled service..."
    sudo systemctl daemon-reload
    sudo systemctl enable "${SERVICE_NAME}"
    sudo systemctl start "${SERVICE_NAME}"
    echo "pi-oled service enabled and started. Check its status with 'sudo systemctl status ${SERVICE_NAME}'."
else
    echo "Error: Failed to create systemd service file."
    exit 1
fi

CONFIG_FILE="/boot/config.txt"
MODULES_FILE="/etc/modules"

# Enable SPI in config.txt
if ! grep -q "dtparam=spi=on" "$CONFIG_FILE"; then
  echo "Adding dtparam=spi=on to $CONFIG_FILE..."
  echo "dtparam=spi=on" >> "$CONFIG_FILE"
  echo "SPI enabled in $CONFIG_FILE."
else
  echo "dtparam=spi=on already present in $CONFIG_FILE."
fi

# Ensure spi-dev module is loaded
if ! grep -q "spi-dev" "$MODULES_FILE"; then
  echo "Adding spi-dev to $MODULES_FILE..."
  echo "spi-dev" >> "$MODULES_FILE"
  echo "spi-dev module added to $MODULES_FILE."
else
  echo "spi-dev already present in $MODULES_FILE."
fi

echo "SPI should now be enabled. A reboot is recommended for changes to take effect."
echo "You can reboot now by running 'sudo reboot'."
