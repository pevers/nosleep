//! Cross-platform library to block the
//! power save functionality in the OS.
//!
//! ```rust
//! # use std::error::Error;
//! # use std::{time::Duration, thread};
//! # use nosleep::*;
//! # fn main() -> Result<(), Box<dyn Error>> {
//!    let nosleep = NoSleep::new()?;
//!    nosleep.prevent_display_sleep()?;
//!    // Depending on the platform, the block will hold
//!    // until either nosleep will be dropped (Linux)
//!    // or the process exits (macOS) or you manually
//!    // call `nosleep.stop()`
//! #  Ok(())
//! # }
//! ```

#[cfg(target_os = "macos")]
pub use nosleep_mac_sys::*;

#[cfg(target_os = "linux")]
pub use nosleep_nix::*;

#[cfg(target_os = "windows")]
pub use nosleep_windows::*;

#[cfg(target_os = "ios")]
pub use nosleep_ios::*;

#[cfg(test)]
mod tests {
    use nosleep_types::NoSleepTrait;

    use crate::*;

    #[test]
    fn test_block_platform_agnostic() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_display_sleep().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2000));
        nosleep.stop().unwrap();
    }
}
