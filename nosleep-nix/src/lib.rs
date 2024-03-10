//! Wrapper utility to block and unblock the Linux power save mode.
//! It uses either the org.gnome.SessionManager D-Bus or the
//! org.freedesktop.PowerManagement API under the hood.
//!
//! Heavily inspired on the Chromium source code:
//! https://chromium.googlesource.com/chromium/src.git/+/refs/heads/main/services/device/wake_lock/power_save_blocker/power_save_blocker_linux.cc

use dbus::blocking::{BlockingSender, Connection};
use nosleep_types::{NoSleepError, NoSleepTrait};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NoSleepType {
    PreventUserIdleDisplaySleep,
    PreventUserIdleSystemSleep,
}

#[derive(Debug, Copy, Clone)]
enum DBusAPI {
    GnomeApi,                  // org.gnome.Sessionmanager
    FreeDesktopPowerApi,       // org.freedesktop.PowerMansagement
    FreeDesktopScreenSaverAPI, // org.freedesktop.ScreenSaver
}

// Inhibit flags defined in the org.gnome.SessionManager interface.
enum GnomeAPIInhibitFlags {
    InhibitSuspendSession = 4,
    InhibitMarkSessionIdle = 8,
}

struct NoSleepHandle {
    // Handle to a locks being held
    handle: u32,
    // The API used to acquire the lock
    api: DBusAPI,
}

pub struct NoSleep {
    // Connection to the D-Bus
    d_bus: Connection,

    // The handles to all the locks
    no_sleep_handles: Vec<NoSleepHandle>,
}

impl NoSleep {
    fn prevent_sleep(&mut self, nosleep_type: NoSleepType) -> Result<(), NoSleepError> {
        // Clear any previous handles held
        self.stop()?;

        let response = self.inhibit(&DBusAPI::GnomeApi, &nosleep_type);
        if let Ok(handle) = response {
            self.no_sleep_handles = vec![handle];
            return Ok(());
        }

        // Try again using the FreeDesktopPowerApi for which we need two calls
        let mut handles: Vec<NoSleepHandle> = vec![];
        if nosleep_type == NoSleepType::PreventUserIdleDisplaySleep {
            let handle = self.inhibit(
                &DBusAPI::FreeDesktopScreenSaverAPI,
                &NoSleepType::PreventUserIdleDisplaySleep,
            )?;
            handles.push(handle);
        }
        // Prevent suspension
        let handle = self.inhibit(&DBusAPI::FreeDesktopPowerApi, &nosleep_type)?;
        handles.push(handle);
        self.no_sleep_handles = handles;
        Ok(())
    }

    fn inhibit(
        &self,
        api: &DBusAPI,
        nosleep_type: &NoSleepType,
    ) -> Result<NoSleepHandle, NoSleepError> {
        let msg = inhibit_msg(api, nosleep_type);
        let response = self
            .d_bus
            .send_with_reply_and_block(msg, std::time::Duration::from_millis(5000))
            .map_err(|e| NoSleepError::DBus {
                reason: e.to_string(),
            })?;
        match response.get1::<u32>() {
            Some(handle) => Ok(NoSleepHandle { handle, api: *api }),
            None => Err(NoSleepError::DBus {
                reason: "Invalid message or type".to_string(),
            }),
        }
    }
}

impl NoSleepTrait for NoSleep {
    /// Creates a new NoSleep type and connects to the D-Bus.
    /// The session is automatically closed when the instance is dropped.
    fn new() -> Result<NoSleep, NoSleepError> {
        Ok(NoSleep {
            d_bus: Connection::new_session().map_err(|e| NoSleepError::Init {
                reason: e.to_string(),
            })?,
            no_sleep_handles: vec![],
        })
    }

    fn prevent_display_sleep(&mut self) -> Result<(), NoSleepError> {
        self.prevent_sleep(NoSleepType::PreventUserIdleDisplaySleep)
    }

    fn prevent_system_sleep(&mut self) -> Result<(), NoSleepError> {
        self.prevent_sleep(NoSleepType::PreventUserIdleSystemSleep)
    }

    fn stop(&mut self) -> Result<(), NoSleepError> {
        for handle in &self.no_sleep_handles {
            let msg = uninhibit_msg(&handle.api, handle.handle);
            self.d_bus
                .send_with_reply_and_block(msg, std::time::Duration::from_millis(5000))
                .map_err(|e| NoSleepError::StopLock {
                    reason: e.to_string(),
                })?;
        }
        Ok(())
    }
}

fn inhibit_msg(api: &DBusAPI, nosleep_type: &NoSleepType) -> dbus::Message {
    match api {
        DBusAPI::GnomeApi => {
            // Arguments are
            // app_id:       application identifier
            // toplevel_xid: toplevel x window identifier
            // reason:       human readable reason
            // flags:        flags that specify what should be inhibited
            let flags = match nosleep_type {
                NoSleepType::PreventUserIdleDisplaySleep => {
                    GnomeAPIInhibitFlags::InhibitMarkSessionIdle as u32
                        | GnomeAPIInhibitFlags::InhibitSuspendSession as u32
                }
                NoSleepType::PreventUserIdleSystemSleep => {
                    GnomeAPIInhibitFlags::InhibitSuspendSession as u32
                }
            };
            dbus::Message::call_with_args(
                "org.gnome.SessionManager",
                "/org/gnome/SessionManager",
                "org.gnome.SessionManager",
                "Inhibit",
                (
                    "org.powersaveblocker.app",
                    0u32,
                    "Power Save Blocker",
                    flags,
                ),
            )
        }
        // The arguments of the method are:
        //  app_id: The application identifier
        //  reason: The reason for the inhibit
        DBusAPI::FreeDesktopPowerApi => dbus::Message::call_with_args(
            "org.freedesktop.PowerManagement",
            "/org/freedesktop/PowerManagement/Inhibit",
            "org.freedesktop.PowerManagement.Inhibit",
            "Inhibit",
            ("org.powersaveblocker.app", "Power Save Blocker"),
        ),
        DBusAPI::FreeDesktopScreenSaverAPI => dbus::Message::call_with_args(
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver",
            "Inhibit",
            ("org.powersaveblocker.app", "Power Save Blocker"),
        ),
    }
}

fn uninhibit_msg(api: &DBusAPI, handle: u32) -> dbus::Message {
    match api {
        DBusAPI::GnomeApi => {
            // Arguments are
            // handle:       lock from the inhibit method
            dbus::Message::call_with_args(
                "org.gnome.SessionManager",
                "/org/gnome/SessionManager",
                "org.gnome.SessionManager",
                "Uninhibit",
                (handle,),
            )
        }
        DBusAPI::FreeDesktopPowerApi => dbus::Message::call_with_args(
            "org.freedesktop.PowerManagement",
            "/org/freedesktop/PowerManagement/Inhibit",
            "org.freedesktop.PowerManagement.Inhibit",
            "UnInhibit",
            (handle,),
        ),
        DBusAPI::FreeDesktopScreenSaverAPI => dbus::Message::call_with_args(
            "org.freedesktop.ScreenSaver",
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver",
            "UnInhibit",
            (handle,),
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_inhibit_gnome_api_message_prevent_display_sleep() {
        let msg = inhibit_msg(
            &DBusAPI::GnomeApi,
            &NoSleepType::PreventUserIdleDisplaySleep,
        );
        assert_eq!("/org/gnome/SessionManager", &*msg.path().unwrap());
        assert_eq!("org.gnome.SessionManager", &*msg.interface().unwrap());
        assert_eq!("Inhibit", &*msg.member().unwrap());
        assert_eq!("org.gnome.SessionManager", &*msg.destination().unwrap());
        assert_eq!(12, msg.get_items().last().unwrap().inner::<u32>().unwrap());
    }

    #[test]
    fn test_inhibit_gnome_api_message_prevent_system_sleep() {
        let msg = inhibit_msg(&DBusAPI::GnomeApi, &NoSleepType::PreventUserIdleSystemSleep);
        assert_eq!("/org/gnome/SessionManager", &*msg.path().unwrap());
        assert_eq!("org.gnome.SessionManager", &*msg.interface().unwrap());
        assert_eq!("Inhibit", &*msg.member().unwrap());
        assert_eq!("org.gnome.SessionManager", &*msg.destination().unwrap());
        assert_eq!(4, msg.get_items().last().unwrap().inner::<u32>().unwrap());
    }

    #[test]
    fn test_uninhibit_gnome_api() {
        let msg = uninhibit_msg(&DBusAPI::GnomeApi, 0);
        assert_eq!("/org/gnome/SessionManager", &*msg.path().unwrap());
        assert_eq!("org.gnome.SessionManager", &*msg.interface().unwrap());
        assert_eq!("Uninhibit", &*msg.member().unwrap());
        assert_eq!("org.gnome.SessionManager", &*msg.destination().unwrap());
        assert_eq!(0, msg.get_items().last().unwrap().inner::<u32>().unwrap());
    }

    #[test]
    fn test_inhibit_freedesktop_screen_saver_api() {
        let msg = inhibit_msg(
            &DBusAPI::FreeDesktopScreenSaverAPI,
            &NoSleepType::PreventUserIdleDisplaySleep,
        );
        assert_eq!("/org/freedesktop/ScreenSaver", &*msg.path().unwrap());
        assert_eq!("org.freedesktop.ScreenSaver", &*msg.interface().unwrap());
        assert_eq!("Inhibit", &*msg.member().unwrap());
        assert_eq!("org.freedesktop.ScreenSaver", &*msg.destination().unwrap());
    }

    #[test]
    fn test_uninhibit_freedesktop_screen_saver_api() {
        let msg = uninhibit_msg(&DBusAPI::FreeDesktopScreenSaverAPI, 0);
        assert_eq!("/org/freedesktop/ScreenSaver", &*msg.path().unwrap());
        assert_eq!("org.freedesktop.ScreenSaver", &*msg.interface().unwrap());
        assert_eq!("UnInhibit", &*msg.member().unwrap());
        assert_eq!("org.freedesktop.ScreenSaver", &*msg.destination().unwrap());
        assert_eq!(0, msg.get_items().last().unwrap().inner::<u32>().unwrap());
    }

    #[test]
    fn test_freedesktop_power_api() {
        let msg = inhibit_msg(
            &DBusAPI::FreeDesktopPowerApi,
            &NoSleepType::PreventUserIdleSystemSleep,
        );
        assert_eq!(
            "/org/freedesktop/PowerManagement/Inhibit",
            &*msg.path().unwrap()
        );
        assert_eq!(
            "org.freedesktop.PowerManagement.Inhibit",
            &*msg.interface().unwrap()
        );
        assert_eq!("Inhibit", &*msg.member().unwrap());
        assert_eq!(
            "org.freedesktop.PowerManagement",
            &*msg.destination().unwrap()
        );
    }

    // Can only run with an active Gnome Session
    #[test]
    #[ignore]
    fn test_start() {
        let mut nosleep = NoSleep::new().unwrap();
        nosleep.prevent_system_sleep().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2000));
        nosleep.stop().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }
}
