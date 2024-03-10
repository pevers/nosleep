use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum NoSleepError {
    #[snafu(display("Could not initialize: {:?}", reason))]
    Init
    {
        reason: String,
    },
    #[snafu(display("Could not prevent sleep: {:?}", reason))]
    PreventSleep
    {
        reason: String,
    },
    #[snafu(display("Could not stop lock: {:?}", reason))]
    StopLock
    {
        reason: String,
    },
}

pub trait NoSleepTrait {
    fn new() -> Result<Self, NoSleepError>
    where
        Self: Sized;

    /// Prevents the display from dimming automatically.
    /// For example: playing a video.
    fn prevent_display_sleep(&mut self) -> Result<(), NoSleepError>;

    /// Prevents the system from sleeping automatically due to a lack of user activity.
    /// For example: downloading a file in the background.
    fn prevent_system_sleep(&mut self) -> Result<(), NoSleepError>;

    /// Cancels any previous call to `prevent_display_sleep` or `prevent_system_sleep`.
    fn stop(&mut self) -> Result<(), NoSleepError>;
}