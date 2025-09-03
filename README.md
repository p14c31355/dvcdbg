<div align="center">
  <h1>dvcdbg</h1>
</div>

> 🛠️ Lightweight debugging crate for embedded Rust (no_std friendly)

`dvcdbg` is a lightweight, `no_std`-friendly debugging library for embedded Rust. It is designed to simplify the initial setup and bring-up of new hardware by providing a convenient set of diagnostic tools.

---

## ✨ Key Features

- ✅ Works in `no_std` environments
- ✅ Lightweight and fast, formatless logging support
- ✅ Includes useful embedded utilities:
  - I2C bus scanner (`scan_i2c`)
  - Hex dump (`write_hex!`)
  - Execution cycle measurement (`measure_cycles!`)
- ✅ Quick diagnostic workflow with `quick_diag!`

---

## 📦 Quickstart

```sh
cargo add dvcdbg --no-default-features --features "ehal_0_2"
```

---

## [Detailed settings](docs/USAGE.md)

---

## Provided Macros

- **Logging & Formatting**
  - `write_hex!(dst, &buf)`  
    Write a byte slice in **hexadecimal** (`12 AB FF `).
  - `write_bin!(dst, &buf)`  
    Write a byte slice in **binary** (`10101010 11110000 `).
  - `assert_log!(cond, logger, "msg")`  
    Log an assertion failure without panicking.

- **Diagnostics**
  - `scan_i2c(i2c, serial)`
    Scan I2C bus and log found devices.
  - `quick_diag!(serial, i2c, timer [, { expr }])`  
    Run a quick diagnostic workflow: serial check, I2C scan, optional cycle measurement.

- **Timing & Control**
  - `measure_cycles!(expr, timer)`  
    Measure execution cycles (or timestamps) for an expression.
  - `loop_with_delay!(delay, ms, { body })`  
    Run a loop with a fixed delay between iterations.

- **Adapters**
  - `adapt_serial!(AdapterName)`  
    Wrap a custom serial-like type to implement:
    - [`core::fmt::Write`]  
    - [`embedded_io::Write`]  
    - [`nb::serial::Write<u8>`]  

- **Explore**
  - See [docs/USAGE_EXPLORE.md](docs/USAGE_EXPLORE.md) for more details.

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
panic = "abort"
codegen-units = 1
lto = true
strip = true
opt-level = "z"
```

---

## 🛠️ Supported Environments

* Rust `no_std`
* AVR (Arduino Uno)
* ESP-IDF / other HALs supported via serial abstraction

---

## 🤝 Contributing

Bug reports, feature suggestions, and pull requests are welcome!
See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

---

## 📄 License

[MIT](docs/LICENSE-MIT) OR [Apache-2.0](docs/LICENSE-APACHE)
