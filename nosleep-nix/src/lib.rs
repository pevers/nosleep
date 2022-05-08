//! Wrapper utility to block and unblock the Linux power save mode.
//! It uses either the org.gnome.SessionManager D-Bus or the
//! org.freedesktop.PowerManagement API under the hood.
//!
//! Heavily inspired on the Chromium source code:
//! https://chromium.googlesource.com/chromium/src.git/+/refs/heads/main/services/device/wake_lock/power_save_blocker/power_save_blocker_linux.cc

use dbus::blocking::{BlockingSender, Connection};
use nosleep_types::NoSleepType;
use snafu::{prelude::*, Backtrace};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("General D-Bus Error"))]
    DBus {
        source: dbus::Error,
        backtrace: Backtrace,
    },

    #[snafu(display("Invalid response from D-Bus"))]
    InvalidResponse { backtrace: Backtrace },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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

struct NoSleepHandleCookie {
    // Handle to a locks being held
    handle: u32,
    // The API used to acquire the lock
    api: DBusAPI,
}

/// Returned by [`NoSleep::start`] to handle
/// the power save block
struct NoSleepHandle<'a> {
    // Connection to the D-Bus
    d_bus: &'a Connection,
    // All the locks that needs cleanup
    cookies: Vec<NoSleepHandleCookie>,
}

impl NoSleepHandle<'_> {
    /// Stop blocking the system from entering power save mode
    pub fn stop(&self) -> Result<()> {
        for cookie in &self.cookies {
            let msg = uninhibit_msg(&cookie.api, cookie.handle);
            self.d_bus
                .send_with_reply_and_block(msg, std::time::Duration::from_millis(5000))
                .context(DBusSnafu)?;
        }
        Ok(())
    }
}

pub struct NoSleep {
    // Connection to the D-Bus
    d_bus: Connection,

    // The unblock handle
    no_sleep_handle: Option<NoSleepHandle<_>>
}

impl NoSleep {
    /// Creates a new NoSleep type and connects to the D-Bus.
    /// The session is automatically closed when the instance is dropped.
    pub fn new() -> Result<NoSleep> {
        Ok(NoSleep {
            d_bus: Connection::new_session().context(DBusSnafu)?,
            no_sleep_handle: None
        })
    }

    /// Blocks the system from entering low-power (sleep) mode.
    /// By making an synchronous call to the D-Bus.
    /// If [`self::stop`] is not called, then he lock will be cleaned up
    /// when the bus is closed.
    pub fn start(&mut self, nosleep_type: NoSleepType) -> Result<()> {
        // Clear any previous handles held
        self.stop()?;

        let response = self.inhibit(&DBusAPI::GnomeApi, &nosleep_type);
        if let Ok(cookie) = response {
            return Ok(NoSleepHandle {
                d_bus: &self.d_bus,
                cookies: vec![cookie],
            });
        }
        // Try again using the FreeDesktopPowerApi (we need two calls)
        let mut cookies: Vec<NoSleepHandleCookie> = vec![];
        if nosleep_type == NoSleepType::PreventUserIdleDisplaySleep {
            let cookie = self.inhibit(&DBusAPI::FreeDesktopScreenSaverAPI, &nosleep_type)?;
            cookies.push(cookie);
        }
        // Prevent suspension
        let cookie = self.inhibit(&DBusAPI::FreeDesktopPowerApi, &nosleep_type)?;
        cookies.push(cookie);
        self.no_sleep_handle = Some(NoSleepHandle {
            d_bus: &self.d_bus,
            cookies,
        });
        Ok(())
    }

    /// Stop blocking the system from entering power save mode
    pub fn stop(&self) -> Result<()> {
        for cookie in &self.cookies {
            let msg = uninhibit_msg(&cookie.api, cookie.handle);
            self.d_bus
                .send_with_reply_and_block(msg, std::time::Duration::from_millis(5000))
                .context(DBusSnafu)?;
        }
        Ok(())
    }

    fn inhibit(&self, api: &DBusAPI, nosleep_type: &NoSleepType) -> Result<NoSleepHandleCookie> {
        let msg = inhibit_msg(api, nosleep_type);
        let response = self
            .d_bus
            .send_with_reply_and_block(msg, std::time::Duration::from_millis(5000))
            .context(DBusSnafu)?;
        let handle = response.get1().context(InvalidResponseSnafu)?;
        Ok(NoSleepHandleCookie { handle, api: *api })
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
            // cookie:       lock from the inhibit method
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
            "org.freedesktop.PowerManagment.Inhibit",
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
        nosleep
            .start(NoSleepType::PreventUserIdleSystemSleep)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2000));
        nosleep.stop().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }
}
