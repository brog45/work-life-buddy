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

use crate::config::{Config, HumanDuration};
use crate::notify;
use anyhow::Result;
use chrono::{DateTime, Duration, Local, NaiveTime, Timelike};
use std::time::Duration as StdDuration;
use tokio::time::sleep;

pub enum CycleResult {
    /// Proceed to log the user out.
    LogOut,
}

/// Returns true when `now` falls within the configured off-hours window.
///
/// Supports windows that span midnight (e.g. 18:00–07:00) as well as same-day
/// windows (e.g. 09:00–17:00).
pub fn is_off_hours(config: &Config, now: &DateTime<Local>) -> bool {
    let h = now.hour();
    if config.off_hours_start > config.off_hours_end {
        // Window spans midnight: off hours if hour >= start OR hour < end
        h >= config.off_hours_start || h < config.off_hours_end
    } else {
        h >= config.off_hours_start && h < config.off_hours_end
    }
}

/// Returns the `StdDuration` until the next off-hours window opens.
pub fn time_until_off_hours(config: &Config, now: &DateTime<Local>) -> StdDuration {
    let start_naive =
        NaiveTime::from_hms_opt(config.off_hours_start, 0, 0).expect("valid off_hours_start");

    let today_start = now
        .date_naive()
        .and_time(start_naive)
        .and_local_timezone(Local)
        .unwrap();

    let target = if today_start > *now {
        today_start
    } else {
        today_start + Duration::days(1)
    };

    (target - *now)
        .to_std()
        .unwrap_or(StdDuration::from_secs(60))
}

/// Run a single logout cycle: show one warning then log out.
pub async fn run_logout_cycle(config: &Config) -> Result<CycleResult> {
    let now = Local::now();
    let logout_duration = config.logout_interval.0;
    let warning_before = config.warning_interval.as_ref().map(|w| w.0).unwrap_or(logout_duration);
    let logout_time = now + Duration::from_std(logout_duration).unwrap_or(Duration::minutes(30));

    log::info!(
        "Logout cycle started — logout scheduled at {}",
        logout_time.format("%H:%M:%S")
    );

    // If warning_interval < logout_interval, sleep until the warning fires.
    let warning_delay = logout_duration.saturating_sub(warning_before);
    if !warning_delay.is_zero() {
        sleep(warning_delay).await;
    }

    let display_secs = warning_before.as_secs().clamp(10, 300) as u32;
    let body = format!(
        "You will be logged out in <b>{}</b>.\nSave your work!",
        HumanDuration(warning_before),
    );

    log::info!("Warning: logging out in {}", HumanDuration(warning_before));

    // note.show() internally blocks on a tokio runtime; must run off the async executor.
    tokio::task::spawn_blocking(move || {
        notify::show_warning(&body, display_secs, true);
    })
    .await?;

    let remaining = (logout_time - Local::now())
        .to_std()
        .unwrap_or(StdDuration::ZERO);
    if !remaining.is_zero() {
        sleep(remaining).await;
    }

    Ok(CycleResult::LogOut)
}

/// Run the configured logout command and log its outcome.
pub async fn execute_logout(config: &Config) -> Result<()> {
    log::info!("Executing logout command: {}", config.logout_command);
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&config.logout_command)
        .output()
        .await?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let level = if output.status.success() { log::Level::Info } else { log::Level::Error };
    log::log!(
        level,
        "Logout command exited {} — stdout: {:?} stderr: {:?}",
        output.status,
        stdout.trim(),
        stderr.trim(),
    );
    Ok(())
}
