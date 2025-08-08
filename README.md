# dvcdbg

> 🛠️ Lightweight debug & logger crate for embedded Rust (no_std friendly)

`dvcdbg` is a lightweight logging and debugging output library for embedded Rust development.  
It can be used in a `no_std` environment and supports log output via UART, I2C, etc.

---

## ✨ Features

- ✅ `no_std` support
- ✅ Formatless, fast and lightweight
- ✅ It also includes utilities for embedded debugging, such as an I2C scanner.

## 📦 Quickstart

```toml
# Cargo.toml
[dependencies]
dvcdbg = { git = "https://github.com/p14c31355/dvcdbg" }
```
```rust
use dvcdbg::logger::SerialLogger;
let mut logger = SerialLogger::new(serial);
logger.log("Init I2C");
```

---

## 📚 Documentation

* [API Documentation (docs.rs)](https://docs.rs/dvcdbg) (Link will be active after publishing)

## 🛠️ Supported environments

* Rust `no_std`
* AVR ( Arduino Uno )

## 🤝 Contributions welcome!

Bug reports, feature suggestions, and pull requests are welcome! Please see our [contribution guidelines](CONTRIBUTING.md).

## 📄 Licenses

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
