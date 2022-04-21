//! Cross-platform library to block the
//! power save functionality in the OS.
//!
//! ```rust
//! # use std::error::Error;
//! # use std::{time::Duration, thread};
//! # use nosleep::*;
//! # fn main() -> Result<(), Box<dyn Error>> {
//!    let nosleep = NoSleep::new().unwrap();
//!    let handle = nosleep.start(NoSleepType::PreventUserIdleDisplaySleep)?;
//!    // Depending on the platform, the block will hold
//!    // until either nosleep will be dropped (Linux)
//!    // or the process exits (macOS)
//! #  Ok(())
//! # }
//! ```

pub use nosleep_types::NoSleepType;

#[cfg(target_os = "macos")]
pub use nosleep_mac_sys::*;

#[cfg(target_os = "linux")]
pub use nosleep_nix::*;

#[cfg(target_os = "windows")]
pub use nosleep_windows::*;

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_block_platform_agnostic() {
        let nosleep = NoSleep::new().unwrap();
        let handle = nosleep
            .start(NoSleepType::PreventUserIdleDisplaySleep)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2000));
        handle.stop().unwrap();
    }
}
