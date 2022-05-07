#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSleepType {
    /// Prevents the display from dimming automatically.
    /// For example: playing a video.
    PreventUserIdleDisplaySleep,
    /// Prevents the system from sleeping automatically due to a lack of user activity.
    /// For example: downloading a file in the background.
    PreventUserIdleSystemSleep,
}
