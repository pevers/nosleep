//! Block the power save functionality on macOS

#![allow(improper_ctypes)]

use std::ops::Deref;

use nosleep_types::{NoSleepError, NoSleepTrait};
use objc_foundation::{INSString, NSString};

mod sys {
    use objc_foundation::NSString;

    #[link(name = "nosleep")]
    extern "C" {
        pub fn start(
            noSleepType: *const NSString,
            handle: *mut std::os::raw::c_uint,
        ) -> std::os::raw::c_int;
        pub fn stop(handle: std::os::raw::c_uint);
    }
}

pub struct NoSleep {
    // The unblock handle
    no_sleep_handle: Option<u32>,
}

impl NoSleepTrait for NoSleep {
    fn new() -> Result<NoSleep, NoSleepError> {
        Ok(NoSleep {
            no_sleep_handle: None,
        })
    }

    fn prevent_display_sleep(&mut self) -> Result<(), NoSleepError> {
        self.stop()?;

        let mut handle = 0u32;
        let ret = unsafe {
            sys::start(
                NSString::from_str("PreventUserIdleDisplaySleep").deref(),
                &mut handle,
            )
        };
        if ret != 0 {
            return Err(NoSleepError::PreventSleep {
                reason: ret.to_string(),
            });
        }
        self.no_sleep_handle = Some(handle);
        Ok(())
    }

    fn prevent_system_sleep(&mut self) -> Result<(), NoSleepError> {
        self.stop()?;

        let mut handle = 0u32;
        let ret = unsafe {
            sys::start(
                NSString::from_str("PreventUserIdleSystemSleep").deref(),
                &mut handle,
            )
        };
        if ret != 0 {
            return Err(NoSleepError::PreventSleep {
                reason: ret.to_string(),
            });
        }
        self.no_sleep_handle = Some(handle);
        Ok(())
    }

    fn stop(&mut self) -> Result<(), NoSleepError> {
        if let Some(handle) = &self.no_sleep_handle {
            unsafe {
                sys::stop(*handle);
            }
            self.no_sleep_handle.take();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nosleep_types::NoSleepTrait;

    use super::NoSleep;

    #[test]
    fn test_start() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_display_sleep().unwrap();
    }

    #[test]
    fn test_stop() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_display_sleep().unwrap();
        nosleep.stop().unwrap();
    }
}
