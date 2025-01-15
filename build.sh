#!/bin/bash

cargo b --bin oled --target arm-unknown-linux-gnueabihf --release
cargo r --bin remote-dev -- client --url "http://192.168.2.60:3000" run --file ./target/arm-unknown-linux-gnueabihf/release/oled
