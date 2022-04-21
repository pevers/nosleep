//! Thin wrapper utility that provides utility
//! methods to block and unblock the macOS power save mode
//! 
//! Example
//! ```rust
//! // Actively block the system sleep mode
//! let handle = start(NoSleepType::PreventUserIdleSystemSleep);
//! if handle.is_none() {
//!     println!("Could not block system power save");
//! }
//! ```
 
#[cfg(target_os = "macos")]
#![allow(improper_ctypes)]

use objc_foundation::{INSString, NSString};
use objc_id::Id;
use std::ops::Deref;

mod sys {
    use objc_foundation::NSString;

    #[link(name = "nosleep")]
    extern "C" {
        pub fn start(noSleepType: *const NSString, handle: *mut std::os::raw::c_uint) -> std::os::raw::c_int;
        pub fn stop(handle: std::os::raw::c_uint);
        pub fn isStarted(handle: std::os::raw::c_uint) -> bool;
    }
}

/// Possible power save modes
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NoSleepType {
    /// Prevents the display from dimming automatically.
    PreventUserIdleDisplaySleep,
    //// Prevents the system from sleeping automatically due to a lack of user activity.
    PreventUserIdleSystemSleep,
}

impl NoSleepType {
    fn as_ns_string(&self) -> Id<NSString> {
        match self {
            NoSleepType::PreventUserIdleDisplaySleep => {
                NSString::from_str("PreventUserIdleDisplaySleep")
            }
            NoSleepType::PreventUserIdleSystemSleep => {
                NSString::from_str("PreventUserIdleSystemSleep")
            }
        }
    }
}

/// Blocks the system from entering low-power (sleep) mode.
/// Returns the handle to the block, or None when not successfull.
pub fn start(no_sleep_type: NoSleepType) -> Option<u32> {
    let mut handle = 0u32;
    unsafe {
        let ret = sys::start(no_sleep_type.as_ns_string().deref(), &mut handle);
        if ret == 0 {
            return Some(handle);
        }
    };
    None
}

/// Stop blocking the system from entering low-power (sleep) mode
/// by providing a [`u32`] from [`start`].
pub fn stop(no_sleep_handle: u32) {
    unsafe {
        sys::stop(no_sleep_handle);
    }
}

/// Checks if the power save block is active
/// for a provided [`u32`] from [`start`]
pub fn is_started(no_sleep_handle: u32) -> bool {
    unsafe { sys::isStarted(no_sleep_handle) }
}

#[cfg(test)]
mod tests {
    use crate::{is_started, start, stop, NoSleepType};

    #[test]
    fn test_start() {
        let ret = start(NoSleepType::PreventUserIdleDisplaySleep);
        assert_eq!(true, ret.is_some());
    }

    #[test]
    fn test_stop() {
        let ret = start(NoSleepType::PreventUserIdleSystemSleep);
        stop(ret.unwrap());
    }

    #[test]
    fn test_is_started() {
        assert_eq!(false, is_started(1));
        let ret = start(NoSleepType::PreventUserIdleDisplaySleep).unwrap();
        assert_eq!(true, is_started(ret));
        stop(ret);
        assert_eq!(false, is_started(ret));
    }
}
