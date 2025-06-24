# Agent Guidelines for pi-oled

## Build/Test Commands
- **Build all**: `cargo build` or `cargo b`
- **Build release**: `cargo build --release`
- **Build for Pi**: `cargo build --bin oled --target arm-unknown-linux-gnueabihf --release`
- **Build and send binary to Rasberry Pi**: `./build.sh`
- **Test**: `cargo test`
- **Test single**: `cargo test test_name`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## Project Structure
- `oled/`: Main Pi OLED display application (Raspberry Pi hardware interface)
- `remote-dev/`: Remote development server/client for cross-compilation deployment

## Code Style
- **Edition**: Rust 2021, nightly toolchain
- **Imports**: Group std, external crates, then local modules with blank lines between
- **Error handling**: Use `anyhow::Result<T>` for fallible functions
- **Async**: Use tokio for async runtime
- **Logging**: Use `tracing` crate (debug, info, warn, error macros)
- **Naming**: snake_case for variables/functions, PascalCase for types/enums
- **Comments**: Use `//` for line comments, `///` for doc comments
