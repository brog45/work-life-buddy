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

use anyhow::{anyhow, Result};
use chrono::Local;
use std::time::Duration;
use tokio::time::sleep;

mod config;
mod notify;
mod scheduler;

fn apply_overrides(config: &mut config::Config, args: &[String]) -> Result<()> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "--verbose" | "--help" => {}
            "--enforce" => config.enforce = true,
            "--no-enforce" => config.enforce = false,
            flag @ ("--warning-interval"
            | "--logout-interval"
            | "--logout-command"
            | "--off-hours-start"
            | "--off-hours-end") => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| anyhow!("{flag} requires a value"))?;
                match flag {
                    "--warning-interval" => config.warning_interval = Some(val.parse().map_err(|e| anyhow!("{e}"))?),
                    "--logout-interval" => config.logout_interval = val.parse().map_err(|e| anyhow!("{e}"))?,
                    "--logout-command" => config.logout_command = val.clone(),
                    "--off-hours-start" => config.off_hours_start = val.parse()?,
                    "--off-hours-end" => config.off_hours_end = val.parse()?,
                    _ => unreachable!(),
                }
            }
            other => anyhow::bail!("Unknown option: {other}"),
        }
        i += 1;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--version") {
        println!("work-life-buddy {} ({})", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--help") {
        println!("work-life-buddy {} ({})\n", env!("CARGO_PKG_VERSION"), env!("GIT_HASH"));
        print!(concat!(
            "Usage: work-life-buddy [OPTIONS]\n",
            "\n",
            "Options:\n",
            "  --help                    Print this help message and exit\n",
            "  --version                 Print version and exit\n",
            "  --verbose                 Enable debug logging\n",
            "  --enforce                 Override config: enable logout enforcement\n",
            "  --no-enforce              Override config: disable logout enforcement (dry run)\n",
            "  --logout-interval <dur>   Override config: time from warning to logout (e.g. 30m, 30s)\n",
            "  --warning-interval <dur>  Override config: how long before logout the warning fires\n",
            "  --logout-command <cmd>    Override config: shell command used to log out\n",
            "  --off-hours-start <hour>  Override config: hour (0-23) when off hours begin\n",
            "  --off-hours-end <hour>    Override config: hour (0-23) when off hours end\n",
        ));
        return Ok(());
    }

    let verbose = args.iter().any(|a| a == "--verbose");
    let default_level = if verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level)).init();

    let mut config = config::load()?;
    apply_overrides(&mut config, &args)?;

    log::info!(
        "work-life-buddy v{} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
    );
    log::debug!("Config: {config:#?}");

    if !config.enforce {
        log::warn!("enforce = false — warnings will show but logout will NOT be executed");
    }

    loop {
        let now = Local::now();

        if scheduler::is_off_hours(&config, &now) {
            match scheduler::run_logout_cycle(&config).await? {
                scheduler::CycleResult::LogOut => {
                    log::info!("Logging out");
                    tokio::task::spawn_blocking(|| {
                        notify::show_info("Work-Life Buddy", "Logging you out. Rest well! 🌙", 5);
                    })
                    .await?;

                    // Brief pause so the notification has time to appear.
                    sleep(Duration::from_secs(3)).await;

                    if config.enforce {
                        if let Err(e) = scheduler::execute_logout(&config).await {
                            log::error!("Logout command failed: {e}");
                        }
                        // Give the session manager time to terminate us.
                        // If the command works we won't reach the next iteration.
                        sleep(Duration::from_secs(15)).await;
                    }
                }
            }
        } else {
            let wait = scheduler::time_until_off_hours(&config, &now);
            log::info!(
                "Outside off hours — sleeping {:.0?} until {:02}:00",
                wait,
                config.off_hours_start,
            );
            sleep(wait).await;
        }
    }
}
