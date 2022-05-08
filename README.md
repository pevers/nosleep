# nosleep

[![Continuous Integration (macOS)](https://github.com/pevers/nosleep/actions/workflows/mac.yml/badge.svg)](https://github.com/pevers/nosleep/actions/workflows/mac.yml) [![Continuous Integration (Linux)](https://github.com/pevers/nosleep/actions/workflows/linux.yaml/badge.svg)](https://github.com/pevers/nosleep/actions/workflows/linux.yaml) [![Continuous Integration (Windows)](https://github.com/pevers/nosleep/actions/workflows/windows.yaml/badge.svg)](https://github.com/pevers/nosleep/actions/workflows/windows.yaml) [![license](https://img.shields.io/crates/l/nosleep?style=flat-square)](https://crates.io/crates/nosleep/) [![version](https://img.shields.io/crates/v/nosleep?style=flat-square)](https://crates.io/crates/nosleep/) ![Crates.io](https://img.shields.io/crates/d/nosleep?style=flat-square)

Cross-platform library to block the power save function in the OS.

```rust
use nosleep::{NoSleep, NoSleepType};
let mut nosleep = NoSleep::new().unwrap();
nosleep
    .start(NoSleepType::PreventUserIdleDisplaySleep)
    .unwrap();
std::thread::sleep(std::time::Duration::from_millis(180_000));
nosleep.stop().unwrap(); // Not strictly needed
```