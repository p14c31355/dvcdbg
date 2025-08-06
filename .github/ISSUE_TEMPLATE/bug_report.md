name: 🐛 バグ報告
description: 実装中に発生したバグを報告します
title: "[BUG] "
labels: ["bug"]
assignees:
  - p14c31355

---

## 概要

<!-- 何が起きたのか？簡潔に記述 -->

## 発生環境

- ターゲット: `___`
- HAL or MCU: `___`
- OS/ビルド: `cargo build` / `trunk` / `avr-hal` など
- `no_std`: true / false
- Feature Flags: `sync` / `async` / `std`

## 再現手順

<!-- できれば main.rs を抜粋して記述 -->
```rust
// 例:
let i2c = ...;
let mut oled = Sh1107gBuilder::new().with_address(0x3C).connect(i2c);
oled.init()?; // ← panic する
```
## 期待した挙動

<!-- 正常時の挙動 -->

## 実際の挙動

<!-- panic, error, 画面出力など -->

## 補足

<!-- 任意 -->

---