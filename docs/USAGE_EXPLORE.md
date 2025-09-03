# Initialization Sequence Explorer API

This document describes the API for automatically exploring and executing initialization sequences over I2C. The API is designed for embedded Rust (`no_std`) environments such as Arduino Uno.

---

## Overview

The Explorer API provides tools to:

* Automatically discover valid I2C addresses of connected devices.
* Generate and execute valid initialization sequences.
* Handle command dependencies and batch execution efficiently.
* Detect cycles or failures in command dependencies.

It is intended for scenarios where the initialization sequence is unknown or needs verification.

---

## Key Structures

### `Explorer<N, MAX_DEPS>`

Holds information about all initialization commands and their dependencies.

* **`nodes`**: Array of `CmdNode` representing commands.
* **`N`**: Maximum number of commands.
* **`MAX_DEPS`**: Maximum number of dependencies per command.

---

### `CmdNode`

Represents a single initialization command node.

| Field       | Type           | Description                      |
| ----------- | -------------- | -------------------------------- |
| `bytes`     | `&'static [u8]` | Command bytes to send over I2C   |
| `deps`      | `&'static [u8]` | List of dependent node indices   |

---

## Key Functions

### `pruning_explorer`

```rust,no_run
pub fn pruning_explorer<I2C, S, const N: usize, const CMD_BUFFER_SIZE: usize, const MAX_DEPS: usize>(
    explorer: &Explorer<N, MAX_DEPS>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
) -> Result<(), ExplorerError>
```

* **Description**: Explores all valid initialization sequences for devices found on the I2C bus. Prunes failing commands automatically.
* **Parameters**:

  * `explorer`: Reference to an `Explorer` containing command nodes.
  * `i2c`: I2C interface implementing `I2cCompat`.
  * `serial`: Serial interface implementing `core::fmt::Write` for logs.
  * `prefix`: Command prefix byte.
* **Returns**: `Ok(())` if all sequences were executed successfully, or an `ExplorerError` on failure.
* **Errors**:

  * `NoValidAddressesFound`
  * `BufferOverflow`
  * `DependencyCycle`
  * `ExecutionFailed`

---

### `one_topological_explorer`

```rust,no_run
pub fn one_topological_explorer<I2C, S, const N: usize, const INIT_SEQUENCE_LEN: usize, const CMD_BUFFER_SIZE: usize, const MAX_DEPS: usize>(
    explorer: &Explorer<N, MAX_DEPS>,
    i2c: &mut I2C,
    serial: &mut S,
    prefix: u8,
) -> Result<(), ExplorerError>
```

* **Description**: Generates a single topological sort of commands and executes it on the first detected device. Useful for testing a single valid initialization sequence.
* **Parameters**: Same as `pruning_explorer`.
* **Returns**: `Ok(())` on success, otherwise an `ExplorerError`.
* **Errors**:

  * `NoValidAddressesFound`
  * `DependencyCycle`
  * `ExecutionFailed`

---

## Macros

### `pruning_sort!`

* **Usage**: Wraps `pruning_explorer` for convenience.

```rust,no_run
pruning_sort!(explorer_instance, &mut i2c, &mut serial, PREFIX, 23, 256, 22);
```

### `get_one_sort!`

* **Usage**: Wraps `one_topological_explorer` for convenience.

```rust,no_run
get_one_sort!(explorer_instance, &mut i2c, &mut serial, PREFIX, 23, 13, 256, 22);
```

---

## Example Usage

```rust,no_run
const PREFIX: u8 = 0x00;
let explorer_instance = nodes! {
    prefix = PREFIX,
    [
        [0xAE],
        [0xD5, 0x51] @ [0],
        [0xA8, 0x3F] @ [1],
        ...
        [0xAF] @ [0] // Display ON
    ]
};

let _ = pruning_sort!(explorer_instance.0, &mut i2c, &mut serial, PREFIX, 23, 256, 22);
```

---

## Notes & Caveats

* Ensure the `CMD_BUFFER_SIZE` is sufficient for batched commands.
* All serial logs use `core::fmt::Write` and may fail silently with `.ok()`.
* Dependency cycles will abort execution to prevent I2C conflicts.
* Devices must respond to I2C scans; otherwise `NoValidAddressesFound` is returned.
* Recommended to add small delays (e.g., `arduino_hal::delay_ms`) between I2C operations on slow MCUs.

---
