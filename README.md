# dvcdbg

> 🛠️ Lightweight debug & logger crate for embedded Rust (no_std friendly)

`dvcdbg` is a lightweight logging and debugging output library for embedded Rust development.  
It can be used in a `no_std` environment and supports log output via UART, I2C, etc.

---

## ✨ Features

- ✅ `no_std` support
- ✅ Formatless, fast and lightweight
- ✅ It also includes utilities for embedded debugging, such as an I2C scanner.
- ✅ Easy to use with simple logging and macros
- ✅ Comes with handy built-in utilities such as an I2C bus scanner
- ✅ Features can be selected with feature flags (e.g., `debug_log`)

## 📦 Quickstart

```toml
# Cargo.toml
[dependencies]
dvcdbg = { git = "https://github.com/p14c31355/dvcdbg", features = ["debug_log"] }
```

## 📄 Usage example (Arduino)

```rust
use arduino_hal::default_serial;
use dvcdbg::logger::SerialLogger;

let dp = arduino_hal::Peripherals::take().unwrap();
let pins = arduino_hal::pins!(dp);
let mut serial = default_serial!(dp, pins, 57600);

let mut logger = SerialLogger::new(&mut serial);
logger.log("Init I2C bus...");

// Use `log!` macro (requires debug_log feature)
log!(logger, "Formatted number: {}", 42);

```

---

## 📚 Documentation

* [API Documentation (docs.rs)](https://docs.rs/dvcdbg)

## 🛠️ Supported environments

* Rust `no_std`
* AVR ( Arduino Uno )

## 🤝 Contributions welcome!

Bug reports, feature suggestions, and pull requests are welcome! Please see our [contribution guidelines](CONTRIBUTING.md).

## 📄 Licenses

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
