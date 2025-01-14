# Raspberry Pi Oled Screen

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
