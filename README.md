# work-life-buddy

A Rust daemon for Pop!_OS / COSMIC (and any systemd-based Linux desktop) that enforces healthy work hours by logging you out during off-hours — with an advance warning so you can save your work first.

## Behaviour

During off hours (default: 6 PM – 7 AM), a logout is scheduled on a repeating interval:

- **Warning** — desktop notification with a beep appears `warning_interval` before logout
- **At logout time** — farewell toast, then the configured logout command runs

## Configuration

The config file is created automatically on first run at:

```
~/.config/work-life-buddy/config.toml
```

```toml
off_hours_start = 18    # hour (0–23) when off hours begin — 6 PM
off_hours_end = 7       # hour (0–23) when off hours end   — 7 AM
logout_interval = "30m" # how long from warning to logout (e.g. "30m" or "30s")
warning_interval = "5m" # how long before logout the warning fires (omit to warn at cycle start)
logout_command = "killall --signal HUP --user $USER cosmic-session"
enforce = true          # set to false for dry-run (warnings only, no logout)
```

Durations accept `m` (minutes) or `s` (seconds). The `s` form is handy for testing.

## Installation

### Prerequisites

- Rust toolchain (`rustup`)
- `paplay` for the beep (from the `pulseaudio-utils` package; works with PipeWire-pulse)

No C dependencies — the notification backend is pure Rust via `zbus`.

### Build and install

```bash
cargo install --path .
```

### Set up the systemd user service

```bash
mkdir -p ~/.config/systemd/user
cp work-life-buddy.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now work-life-buddy
```

The service starts automatically with your graphical session and restarts on failure.

### Useful commands

```bash
# View live logs
journalctl --user -u work-life-buddy -f

# Temporarily disable (re-enables on next login)
systemctl --user stop work-life-buddy

# Disable permanently
systemctl --user disable --now work-life-buddy
```

## Building from source

```bash
cargo build --release
# Binary at: target/release/work-life-buddy
```

Set `RUST_LOG=debug` for verbose output.

## License

GPL-3.0-or-later

See [LICENSE](LICENSE) for the full text.
