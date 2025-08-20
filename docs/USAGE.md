# USAGE.md

# dvcdbg Usage Guide

`dvcdbg` is a Rust embedded debugging crate designed to simplify serial debugging and I2C scanning, supporting both `embedded-hal` 0.2 and 1.0.

---

## 1. Feature Flags

The crate uses **Cargo features** to enable different functionality and HAL versions.

| Feature      | Description                                      |
| ------------ | ------------------------------------------------ |
| `logger`     | Enables logging support via serial adapters      |
| `scanner`    | Enables I2C scanner utilities                    |
| `macros`     | Enables macros like `adapt_serial!`              |
| `ehal_0_2`   | Use `embedded-hal` 0.2.x                         |
| `ehal_1_0`   | Use `embedded-hal` 1.0.x                         |

**Default features**: `logger, scanner, macros, ehal_1_0`

> To switch HAL versions, disable the default and explicitly enable:
>
> ```toml
> [dependencies.dvcdbg]
> version = "x.y.z"
> default-features = false
> features = ["ehal_0_2", "logger", "scanner", "macros"]
> ```

---

## 2. `adapt_serial!` Usage Examples

The `adapt_serial!` macro creates a bridge between a serial peripheral and the `embedded_io::Write` + `core::fmt::Write` traits.

### Arduino HAL (AVR)

```rust,no_run
#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;
use core::fmt::Write;

use dvcdbg::prelude::*;
adapt_serial!(UnoSerial);

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut logger = UnoSerial(serial);

    writeln!(logger, "Hello from dvcdbg on Arduino Uno!").unwrap();
    logger.write_all(&[0xDE, 0xAD, 0xBE, 0xEF]).unwrap();

    loop {}
}

```

---

### Custom Serial-Compatible Types

```rust,no_run
use dvcdbg::adapt_serial;
use core::fmt::Write;
use core::convert::Infallible;
use nb;
use embedded_io::Write;

// Any Serial-Compatible Type
struct MySerial;

// Example implementation of `embedded_hal_0_2::serial::Write`
impl embedded_hal_0_2::serial::Write<u8> for MySerial {
    type Error = Infallible;
    fn write(&mut self, _byte: u8) -> nb::Result<(), Self::Error> { Ok(()) }
    fn flush(&mut self) -> nb::Result<(), Self::Error> { Ok(()) }
}

// adapt_serial! Just wrap it in a macro
adapt_serial!(MyAdapter);

let mut uart = MyAdapter(MySerial);

// Write using core::fmt::Write
writeln!(uart, "Hello via custom serial").unwrap();

// Write to the buffer using embedded_io::Write
uart.write_all(&[0xAA, 0xBB]).unwrap();
```

---

## Notes

* **AVR HAL / e-hal 1.0**: The AVR HAL's `Usart` only implements the 0.2 traits, but `adapt_serial!` internally selects the appropriate SerialCompat implementation.
* **0.2 / 1.0 Internal Switching**: The appropriate trait is automatically selected based on the `ehal_0_2` / `ehal_1_0` feature flags.

---

## FAQ

**Q: How do I switch between blocking and non-blocking?**
A: `adapt_serial!` automatically switches between them depending on the feature flag. Users don't need to worry about it.

**Q: Can I specify multiple features?**
A: Only enable one HAL, either `ehal_0_2` or `ehal_1_0`. Other `logger` / `scanner` / `macros` can be combined.

---
