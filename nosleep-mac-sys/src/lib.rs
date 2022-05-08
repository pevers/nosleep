//! Thin wrapper utility that provides utility
//! methods to block and unblock the macOS power save mode

#![allow(improper_ctypes)]

use std::ops::Deref;

use nosleep_types::NoSleepType;
use objc_foundation::{INSString, NSString};
use objc_id::Id;
use snafu::{prelude::*, Backtrace};
mod sys {
    use objc_foundation::NSString;

    #[link(name = "nosleep")]
    extern "C" {
        pub fn start(
            noSleepType: *const NSString,
            handle: *mut std::os::raw::c_uint,
        ) -> std::os::raw::c_int;
        pub fn stop(handle: std::os::raw::c_uint);
        //pub fn isStarted(handle: std::os::raw::c_uint) -> bool;
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not prevent power save mode for option {:?}", option))]
    PreventPowerSaveMode {
        option: NoSleepType,
        backtrace: Backtrace,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

fn nosleep_ns_string(nosleep_type: &NoSleepType) -> Id<NSString> {
    match nosleep_type {
        NoSleepType::PreventUserIdleDisplaySleep => {
            NSString::from_str("PreventUserIdleDisplaySleep")
        }
        NoSleepType::PreventUserIdleSystemSleep => NSString::from_str("PreventUserIdleSystemSleep"),
    }
}

/// Returned by [`NoSleep::start`] to handle
/// the power save block
struct NoSleepHandle {
    handle: u32,
}

impl NoSleepHandle {
    /// Stop blocking the system from entering power save mode
    pub fn stop(self: &NoSleepHandle) -> Result<()> {
        unsafe {
            sys::stop(self.handle);
        }
        Ok(())
    }
}
pub struct NoSleep {
    // The unblock handle
    no_sleep_handle: Option<NoSleepHandle>
}

impl NoSleep {
    pub fn new() -> Result<NoSleep> {
        Ok(NoSleep {
            no_sleep_handle: None
        })
    }

    /// Blocks the system from entering low-power (sleep) mode by
    /// making a synchronous call to the macOS `IOPMAssertionCreateWithName` system call.
    /// If [`self::stop`] is not called, then he lock will be cleaned up
    /// when the process PID exits.
    pub fn start(&mut self, nosleep_type: NoSleepType) -> Result<()> {
        // Clear any previous handles held
        self.stop()?;

        let mut handle = 0u32;
        let ret = unsafe {
            sys::start(nosleep_ns_string(&nosleep_type).deref(), &mut handle)
        };
        if ret != 0 {
            return PreventPowerSaveModeSnafu {
                option: nosleep_type,
            }
            .fail();
        }
        self.no_sleep_handle = Some(NoSleepHandle { handle });
        Ok(())
    }

    /// Stop blocking the system from entering power save mode
    pub fn stop(&self) -> Result<()> {
        if let Some(handle) = &self.no_sleep_handle {
            handle.stop()?; 
        }
        Ok(())
    }
}

/// TODO: Check if this still fits within the API
/// Checks if the power save block is active
/// for a provided [`u32`] from [`start`]
// pub fn is_started(no_sleep_handle: u32) -> bool {
//     unsafe { sys::isStarted(no_sleep_handle) }
// }

#[cfg(test)]
mod tests {
    use crate::{NoSleep, NoSleepType};

    #[test]
    fn test_start() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep
            .start(NoSleepType::PreventUserIdleDisplaySleep)
            .unwrap();
    }

    #[test]
    fn test_stop() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep
            .start(NoSleepType::PreventUserIdleDisplaySleep)
            .unwrap();
        nosleep.stop().unwrap();
    }

    // #[test]
    // fn test_is_started() {
    //     assert_eq!(false, is_started(1));
    //     let ret = start(NoSleepType::PreventUserIdleDisplaySleep).unwrap();
    //     assert_eq!(true, is_started(ret));
    //     stop(ret);
    //     assert_eq!(false, is_started(ret));
    // }
}
