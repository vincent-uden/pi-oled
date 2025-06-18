# Raspberry Pi Oled Screen

## Running the setup script
```sh
wget -qO- https://raw.githubusercontent.com/vincent-uden/pi-oled/refs/heads/master/setup.sh | bash
```

## Remote Development
Example usage:
```sh
cargo r --bin remote-dev -- client --url "http://192.168.2.60:3000/upload" --file ./Cargo.toml        
```

On the server:
```sh
cargo r --bin remote-dev -- server --port 3000
```

Cross-compile for ARM:
```sh
cargo b --bin remote-dev --target arm-unknown-linux-gnueabihf --release
scp ./target/arm-unknown-linux-gnueabihf/release/remote-dev vincent@192.168.2.60:~/remote-dev
```
