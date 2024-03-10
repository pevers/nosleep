//! Thin wrapper utility that provides utility
//! methods to block and unblock the Windows power save mode
//!
//! Inspired on the Chromium source code
//! https://chromium.googlesource.com/chromium/src/+/87cd0848a0d1453e7553a72b0686d42fabf8ff3a/device/power_save_blocker/power_save_blocker_win.cc

use nosleep_types::{NoSleepError, NoSleepTrait};
use windows::core::PWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired,
    PowerSetRequest, POWER_REQUEST_TYPE,
};
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NoSleepType {
    PreventUserIdleDisplaySleep,
    PreventUserIdleSystemSleep,
}

trait IntoPWSTR {
    fn into_pwstr(self) -> (PWSTR, Vec<u16>);
}

impl IntoPWSTR for &str {
    fn into_pwstr(self) -> (PWSTR, Vec<u16>) {
        let mut encoded = self.encode_utf16().chain([0u16]).collect::<Vec<u16>>();

        (PWSTR(encoded.as_mut_ptr()), encoded)
    }
}
impl IntoPWSTR for String {
    fn into_pwstr(self) -> (PWSTR, Vec<u16>) {
        let mut encoded = self.encode_utf16().chain([0u16]).collect::<Vec<u16>>();

        (PWSTR(encoded.as_mut_ptr()), encoded)
    }
}

/// Returned by [`NoSleep::start`] to handle
/// the power save block
pub struct NoSleepHandle {
    // Handle to the PowerRequestSystemRequired block
    system_handle: HANDLE,
    // Handle to the PowerRequestDisplayRequired block
    display_handle: Option<HANDLE>,
}

pub struct NoSleep {
    // Handle to unlock the power save block
    no_sleep_handle: Option<NoSleepHandle>,
}

fn create_power_request(power_request_type: POWER_REQUEST_TYPE) -> Result<HANDLE, NoSleepError> {
    let reason = REASON_CONTEXT {
        Version: 0,
        Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
        Reason: REASON_CONTEXT_0 {
            SimpleReasonString: "Power Save Blocker".into_pwstr().0,
        },
    };
    unsafe {
        let handle = PowerCreateRequest(&reason).map_err(|e| NoSleepError::PreventSleep {
            reason: e.to_string(),
        })?;
        PowerSetRequest(handle, power_request_type).map_err(|e| NoSleepError::PreventSleep {
            reason: e.to_string(),
        })?;
        Ok(handle)
    }
}

impl NoSleep {
    /// Blocks the system from entering low-power (sleep) mode by
    /// making a call to the Windows `PowerCreateRequest`/`PowerSetRequest` system call.
    /// If [`self::stop`] is not called, then he lock will be cleaned up
    /// when NoSleep is dropped.
    fn prevent_sleep(&mut self, nosleep_type: NoSleepType) -> Result<(), NoSleepError> {
        // Clear any previous lock held
        self.stop()?;

        // TODO:
        // PowerRequestSystemRequired implies PowerRequestExsecutionRequired
        // So we don't have to check the Windows version?
        let system_handle = create_power_request(PowerRequestSystemRequired)?;
        let display_handle = if nosleep_type == NoSleepType::PreventUserIdleDisplaySleep {
            create_power_request(PowerRequestDisplayRequired).ok()
        } else {
            None
        };
        self.no_sleep_handle = Some(NoSleepHandle {
            system_handle,
            display_handle,
        });
        Ok(())
    }
}

impl NoSleepTrait for NoSleep {
    fn new() -> Result<NoSleep, NoSleepError> {
        Ok(NoSleep {
            no_sleep_handle: None,
        })
    }

    fn prevent_display_sleep(&mut self) -> Result<(), NoSleepError> {
        self.prevent_sleep(NoSleepType::PreventUserIdleDisplaySleep)
    }

    fn prevent_system_sleep(&mut self) -> Result<(), NoSleepError> {
        self.prevent_sleep(NoSleepType::PreventUserIdleSystemSleep)
    }

    fn stop(&mut self) -> Result<(), NoSleepError> {
        if let Some(handle) = &self.no_sleep_handle {
            unsafe {
                PowerClearRequest(handle.system_handle, PowerRequestSystemRequired).map_err(
                    |e| NoSleepError::StopLock {
                        reason: e.to_string(),
                    },
                )?;
                if let Some(display_handle) = handle.display_handle {
                    PowerClearRequest(display_handle, PowerRequestDisplayRequired).map_err(
                        |e| NoSleepError::StopLock {
                            reason: e.to_string(),
                        },
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_sleep() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_display_sleep().unwrap();
    }

    #[test]
    fn test_system_sleep() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_system_sleep().unwrap();
    }

    #[test]
    fn test_stop() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_display_sleep().unwrap();
        nosleep.stop().unwrap();
    }
}
