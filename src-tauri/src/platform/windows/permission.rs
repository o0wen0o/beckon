//! Windows asks for nothing before `SendInput` reaches another process, so
//! there is no permission to report and no repair path to offer (ADR-0013).

use crate::platform::InputPermission;

/// Nothing to grant, so there is nowhere to send anyone.
pub fn settings_url() -> Option<&'static str> {
    None
}

pub fn input_permission() -> InputPermission {
    InputPermission::NotRequired
}
