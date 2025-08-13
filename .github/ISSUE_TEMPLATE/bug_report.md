---
name: "\U0001F41B Bug Report"
about: Bug Report
title: " "
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
scan_i2c(&mut i2c, &mut logger); // <- failed
```
## Expected behaviour

<!-- Normal behaviour -->

## Actual behaviour

<!-- panic, error, screen output, etc. -->

## Supplementary information

<!-- optional, e.g. screenshots, videos, etc. -->

---
