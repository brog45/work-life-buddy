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

use anyhow::{Context, Result};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// A duration that serializes/deserializes as "30m" or "30s".
#[derive(Clone)]
pub struct HumanDuration(pub Duration);

impl Default for HumanDuration {
    fn default() -> Self {
        Self(Duration::from_secs(30 * 60))
    }
}

impl fmt::Debug for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0.as_secs();
        if secs % 60 == 0 {
            write!(f, "{}m", secs / 60)
        } else {
            write!(f, "{}s", secs)
        }
    }
}

impl Serialize for HumanDuration {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl std::str::FromStr for HumanDuration {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(n) = s.strip_suffix('m') {
            let mins: u64 = n.trim().parse().map_err(|e| format!("{e}"))?;
            Ok(Self(Duration::from_secs(mins * 60)))
        } else if let Some(n) = s.strip_suffix('s') {
            let secs: u64 = n.trim().parse().map_err(|e| format!("{e}"))?;
            Ok(Self(Duration::from_secs(secs)))
        } else {
            Err(format!("expected duration like \"30m\" or \"30s\", got {s:?}"))
        }
    }
}

impl<'de> Deserialize<'de> for HumanDuration {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Hour (0–23) when off hours begin. Default: 18 (6 PM)
    pub off_hours_start: u32,
    /// Hour (0–23) when off hours end. Default: 7 (7 AM)
    pub off_hours_end: u32,
    /// Total cycle length — how long from warning to logout. E.g. "30m" or "30s".
    pub logout_interval: HumanDuration,
    /// How long before logout to show the warning. E.g. "5m" or "10s".
    /// Omit to warn at cycle start (same as logout_interval).
    #[serde(default)]
    pub warning_interval: Option<HumanDuration>,
    /// Shell command used to log out. Runs via `sh -c`.
    /// Available env vars: $USER, $XDG_SESSION_ID, $HOME, etc.
    /// Alternatives:
    ///   COSMIC: systemctl --user stop cosmic-session.target
    ///   General: loginctl terminate-user $USER
    pub logout_command: String,
    /// Set to false to show warnings without actually logging out (dry-run mode).
    pub enforce: bool,
}

impl Default for Config {
    fn default() -> Self {
        let interval = HumanDuration(Duration::from_secs(30 * 60));
        Self {
            off_hours_start: 18,
            off_hours_end: 7,
            warning_interval: None,
            logout_interval: interval,
            logout_command: "killall --signal HUP --user $USER cosmic-session".to_string(),
            enforce: true,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
        })
        .join("work-life-buddy")
        .join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();

    if !path.exists() {
        let config = Config::default();
        let parent = path.parent().expect("config path has parent");
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir {}", parent.display()))?;
        let content = toml::to_string_pretty(&config)?;
        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write default config to {}", path.display()))?;
        log::info!("Created default config at {}", path.display());
        return Ok(config);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("Failed to parse TOML from {}", path.display()))
}
