//! Thin wrapper utility that provides utility
//! methods to block and unblock the Windows power save mode
//!
//! Inspired on the Chromium source code
//! https://chromium.googlesource.com/chromium/src/+/87cd0848a0d1453e7553a72b0686d42fabf8ff3a/device/power_save_blocker/power_save_blocker_win.cc

use nosleep_types::NoSleepType;
use snafu::{prelude::*, Backtrace};
use windows::core::PWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired,
    PowerSetRequest, POWER_REQUEST_TYPE,
};
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not prevent power save mode for option {:?}", option))]
    PreventPowerSaveMode {
        option: POWER_REQUEST_TYPE,
        backtrace: Backtrace,
    },

    #[snafu(display("Could not clear power save mode for option {:?}", option))]
    ClearPowerSaveMode {
        option: POWER_REQUEST_TYPE,
        backtrace: Backtrace,
    },

    #[snafu(display("Could not create a PowerRequest"))]
    CouldNotCreatePowerRequest {
        source: windows::core::Error,
        backtrace: Backtrace,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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

impl NoSleepHandle {
    /// Stop blocking the system from entering power save mode
    pub fn stop(self: &NoSleepHandle) -> Result<()> {
        unsafe {
            if !PowerClearRequest(&self.system_handle, PowerRequestSystemRequired).as_bool() {
                return ClearPowerSaveModeSnafu {
                    option: PowerRequestSystemRequired,
                }
                .fail();
            }
            if !&self
                .display_handle
                .map(|h| PowerClearRequest(&h, PowerRequestDisplayRequired).as_bool())
                .unwrap_or(true)
            {
                return ClearPowerSaveModeSnafu {
                    option: PowerRequestDisplayRequired,
                }
                .fail();
            }
        }
        Ok(())
    }
}

pub struct NoSleep {
    // Handle to unlock the power save block
    no_sleep_handle: Option<NoSleepHandle>,
}

fn create_power_request(power_request_type: POWER_REQUEST_TYPE) -> Result<HANDLE> {
    let reason = REASON_CONTEXT {
        Version: 0,
        Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
        Reason: REASON_CONTEXT_0 {
            SimpleReasonString: "Power Save Blocker".into_pwstr().0,
        },
    };
    unsafe {
        let handle = PowerCreateRequest(&reason).context(CouldNotCreatePowerRequestSnafu)?;
        if PowerSetRequest(&handle, power_request_type).as_bool() {
            return Ok(handle);
        }
        PreventPowerSaveModeSnafu {
            option: power_request_type,
        }
        .fail()
    }
}

impl NoSleep {
    pub fn new() -> Result<NoSleep> {
        Ok(NoSleep {
            no_sleep_handle: None,
        })
    }

    /// Blocks the system from entering low-power (sleep) mode by
    /// making a call to the Windows `PowerCreateRequest`/`PowerSetRequest` system call.
    /// If [`self::stop`] is not called, then he lock will be cleaned up
    /// when NoSleep is dropped.
    pub fn start(&self, nosleep_type: NoSleepType) -> Result<()> {
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
}
