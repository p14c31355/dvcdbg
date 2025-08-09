---
name: "\U0001F41B Bug Report"
about: Bug Report
title: "[BUG] "
labels: bug
assignees: p14c31355

---

## Overview

<!-- What happened? A concise description of the problem. -->

## Environment of occurrence

- Target: `___`
- HAL or MCU: `___`
- OS/Build Tool: `cargo build` / `trunk` / `avr-hal` etc.
- `no_std`: true / false
- Feature Flags: `sync` / `async` / `std`

## Reproduction procedure

<!-- If possible, describe an excerpt from main.rs. -->
```rust
// example:
let i2c = ...;
let mut oled = Sh1107gBuilder::new().with_address(0x3C).connect(i2c);
oled.init()?; // ← panic
```
## Expected behaviour

<!-- Normal behaviour -->

## Actual behaviour

<!-- panic, error, screen output, etc. -->

## Supplementary information

<!-- optional, e.g. screenshots, videos, etc. -->

---
