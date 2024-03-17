//! Block the power save functionality on iOS

use nosleep_types::{NoSleepError, NoSleepTrait};
use objc::runtime::{NO, YES};
use objc::{class, msg_send, sel, sel_impl};

pub struct NoSleep {}

impl NoSleepTrait for NoSleep {
    fn new() -> Result<NoSleep, NoSleepError> {
        Ok(NoSleep {})
    }

    fn prevent_display_sleep(&mut self) -> Result<(), NoSleepError> {
        unsafe {
            let ui_app: *mut objc::runtime::Object =
                msg_send![class!(UIApplication), sharedApplication];
            let _: () = msg_send![ui_app, setIdleTimerDisabled: YES];
        }
        Ok(())
    }

    /// iOS does not have a system sleep prevention API.
    /// This will do the same as prevent_display_sleep
    fn prevent_system_sleep(&mut self) -> Result<(), NoSleepError> {
        self.prevent_display_sleep()
    }

    fn stop(&mut self) -> Result<(), NoSleepError> {
        unsafe {
            let ui_app: *mut objc::runtime::Object =
                msg_send![class!(UIApplication), sharedApplication];
            let _: () = msg_send![ui_app, setIdleTimerDisabled: NO];
        }
        Ok(())
    }
}
