// work-life-buddy — Work-life balance enforcer that logs you out during off hours
// Copyright (C) 2026 Brian Rogers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use notify_rust::{Notification, Timeout, Urgency};

const BELL_SOUND: &str = "/usr/share/sounds/freedesktop/stereo/bell.oga";

fn beep() {
    match std::process::Command::new("paplay").arg(BELL_SOUND).status() {
        Ok(s) if !s.success() => log::warn!("paplay exited {s}"),
        Err(e) => log::warn!("Failed to run paplay: {e}"),
        _ => {}
    }
}

/// Show a warning notification (no action buttons).
///
/// Returns immediately after sending the notification; does not wait for
/// user interaction. `display_secs` controls the auto-dismiss timeout.
pub fn show_warning(body: &str, display_secs: u32, urgent: bool) {
    beep();

    let mut note = Notification::new();
    note.summary("Work-Life Buddy")
        .body(body)
        .timeout(Timeout::Milliseconds(display_secs.saturating_mul(1000)));

    if urgent {
        note.urgency(Urgency::Critical);
    } else {
        note.urgency(Urgency::Normal);
    }

    if let Err(e) = note.show() {
        log::warn!("Failed to show notification: {e}");
    }
}

/// Show a transient informational notification (no action buttons).
pub fn show_info(summary: &str, body: &str, display_secs: u32) {
    beep();

    if let Err(e) = Notification::new()
        .summary(summary)
        .body(body)
        .timeout(Timeout::Milliseconds(display_secs.saturating_mul(1000)))
        .show()
    {
        log::warn!("Failed to show info notification: {e}");
    }
}
