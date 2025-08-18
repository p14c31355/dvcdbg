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
| `quick_diag` | Enables `logger + scanner + macros` together     |
| `ehal_0_2`   | Use `embedded-hal` 0.2.x (blocking/non-blocking) |
| `ehal_1_0`   | Use `embedded-hal` 1.0.x + `nb` (non-blocking)   |

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
use arduino_hal::prelude::*;
use dvcdbg::adapt_serial;
use core::fmt::Write;
use embedded_io::Write;

adapt_serial!(UsartAdapter, write = write, flush = flush);

let dp = arduino_hal::Peripherals::take().unwrap();
let pins = arduino_hal::pins!(dp);
let serial = arduino_hal::default_serial!(dp, pins, 57600);
let mut dbg_uart = UsartAdapter(serial);

writeln!(dbg_uart, "Hello from embedded-io bridge!").unwrap();
dbg_uart.write_all(&[0x01, 0x02, 0x03]).unwrap();
```

### Custom Serial-Like Type

```rust,no_run
use dvcdbg::adapt_serial;
use core::fmt::Write;
use core::convert::Infallible;
use nb;
use embedded_io::Write;

struct MySerial;

impl nb::serial::Write<u8> for MySerial {
    type Error = Infallible;
    fn write(&mut self, _byte: u8) -> nb::Result<(), Self::Error> { Ok(()) }
    fn flush(&mut self) -> nb::Result<(), Self::Error> { Ok(()) }
}

adapt_serial!(MyAdapter, write = write, flush = flush);

let mut uart = MyAdapter(MySerial);
writeln!(uart, "Hello via custom serial").unwrap();
uart.write_all(&[0xAA, 0xBB]).unwrap();
```

---

## 3. Notes and Caveats

* **Optional `flush`**: You may omit the `flush` argument if your peripheral does not implement it.
* **AVR and e-hal 1.0**: AVR HALs may require a **wrapper** to implement the 1.0 traits (`nb::Write<u8>`), because the original HAL only implements 0.2 traits.
* **0.2 / 1.0 internal switching**: The macro automatically selects the correct trait implementation based on the enabled feature flag.

---

## 4. FAQ

**Q: Is `flush` mandatory?**
A: No, it is optional. Only needed if your HAL exposes a flush method.

**Q: How do I know if I should use blocking or non-blocking?**
A: `adapt_serial!` handles both automatically via feature flags. For `ehal_1_0`, non-blocking is assumed.

**Q: Can I mix features?**
A: You should enable exactly one HAL feature: either `ehal_0_2` or `ehal_1_0`. Other features like `logger`, `scanner`, `macros` can be combined freely.

---