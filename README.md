<div align="center">
  <h1>dvcdbg</h1>
</div>

> 🛠️ Lightweight logging & debugging crate for embedded Rust (no_std friendly)

`dvcdbg` is a lightweight, `no_std`-friendly logging and debugging library for embedded Rust. It is designed to simplify the initial setup and bring-up of new hardware by providing a convenient set of diagnostic tools.

---

## ✨ Key Features

- ✅ Works in `no_std` environments
- ✅ Lightweight and fast, formatless logging support
- ✅ Includes useful embedded utilities:
  - I2C bus scanner (`scan_i2c!`)
  - Hex dump (`write_hex!`)
  - Execution cycle measurement (`measure_cycles!`)
- ✅ Quick diagnostic workflow with `quick_diag!`
- ✅ Serial logger abstraction for various HALs
- ✅ Feature flags allow selective compilation:
  - `logger` → logging utilities
  - `scanner` → I2C scanning utilities
  - `macros` → helper macros like `impl_fmt_write_for_serial!`
  - `quick_diag` → workflow macros combining logger + scanner + timing

---

## 📦 Quickstart

```toml
# Cargo.toml
[dependencies]
dvcdbg = { version = "0.1.1", features = ["quick_diag"] }
```

---

## 📄 Usage Example (Arduino)

```rust
use arduino_hal::default_serial;
use dvcdbg::prelude::*;

fn main() {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = default_serial!(dp, pins, 57600);
    let mut logger = SerialLogger::new(&mut serial);

    #[cfg(feature = "quick_diag")]
    {
        // Initialization I2C instance and timer
        let i2c = arduino_hal::I2c::new(
            dp.TWI,
            pins.a4.into_pull_up_input(),
            pins.a5.into_pull_up_input(),
            100_000,
        );
        let timer = arduino_hal::hal::timer::Timer1::new(dp.TC1, arduino_hal::hal::clock::Clock::new());

        // Quick diagnostic: scans I2C bus and measures cycles for test code
        quick_diag!(logger, i2c, timer, {
            your_test_code();
        });
    }
}

fn your_test_code() {
    // 
}

```

---

## 📚 Macros Included

* `impl_fmt_write_for_serial!` → implement `core::fmt::Write` for any serial type
* `write_hex!` → print byte slices in hexadecimal format
* `measure_cycles!` → measure execution cycles or timestamps
* `loop_with_delay!` → loop with fixed delay for testing
* `assert_log!` → log assertions without panicking
* `scan_i2c!` → scan I²C bus for connected devices
* `quick_diag!` → all-in-one diagnostic workflow

---

## 📚 Documentation

* [API Documentation (docs.rs)](https://docs.rs/dvcdbg)

---

## 🚀 Binary Size Optimisation

Since `dvcdbg` is designed for a `no_std` environment, it is important to minimise the final binary size.

Enabling **LTO (link-time optimisation)** and **strip** during release builds will remove unused code from `dvcdbg` and other dependent crates, significantly reducing the binary size.

Add the following settings to your application's `Cargo.toml`.

```toml
# Cargo.toml (application)
[profile.release]
lto = true
strip = true
```

---

## 🛠️ Supported Environments

* Rust `no_std`
* AVR (Arduino Uno)
* ESP-IDF / other HALs supported via serial abstraction

---

## 🤝 Contributing

Bug reports, feature suggestions, and pull requests are welcome!
See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 📄 License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
