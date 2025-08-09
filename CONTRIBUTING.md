---

# Contribution Guidelines

Welcome! We welcome your contributions to this repository. 🙌
Please follow the guidelines below when reporting bugs, adding features, or making improvements.

---

## 🔧 Setting up the development environment

### Prerequisite environment

- Rust (Latest version recommended)
- `cargo` / `rustup` already installed
- For embedded targets, a target-specific toolchain is also required (e.g., `avr-hal`, `esp-idf`, `thumbv7em`).

```sh
# If you need Rust nightly
rustup install nightly
```

### Getting dependencies

```sh
cargo check
```

---

## 🐛 Bug report

Please create an issue following the [Bug Report Template](.github/ISSUE_TEMPLATE/bug_report.md).
If possible, please attach **reproducible code** and **I2C logs**.

---

## ✨ Feature proposal

Please use the [Feature Request Template](.github/ISSUE_TEMPLATE/feature_request.md) to submit your proposal as an issue.

* Please note the compatibility and limitations with existing drivers.
* Please clearly indicate support for `no_std`.
* Please provide a rationale for any changes to command specifications or additions of macros.

---

## 🔃 Pull request

1. **Create an issue and then create a branch**
   Branch name examples: `fix/init-error`, `feat/drawtarget-support`

2. **Passing tests and `cargo check`**

3. **Describe the explanation according to the PR template.**

4. Please write in the PR comment to close the related issue:

   ```text
   Closes #42
   ```

---

## 🧪 Testing Policy

* The basic requirement is that `cargo test` passes.
* Compilation passes on the `no_std` target.
* Actual machine testing (I2C/SPI) is not subject to CI depending on the environment (visual confirmation is OK).

---

## 📦 Coding conventions

* Compliant with `rustfmt`
* Recommends no warnings for `clippy`
* When both `std` and `no_std` are supported, use `#[cfg(feature = ‘std’)]`.

---

## 🤝 License

This project is licensed under the MIT License and the Apache 2.0 License.
Contributed code will also be released under the MIT License and the Apache 2.0 License.

---

## 💬 Contact

* Maintainer: [p14c31355](https://github.com/p14c31355)
* Feel free to visit Issues or Discussions!

---