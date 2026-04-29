# Rotation Lock

A small Windows utility that **forces laptop mode** on convertible laptops — devices with both laptop and tablet form factors.

## The problem it solves

Convertible laptops use a built-in orientation sensor to auto-rotate the screen between landscape (laptop) and portrait or inverted modes (tablet). Most of these devices give you no real way to disable that behavior — the screen flips at the slightest tilt. Lean back on the couch, lie down to read, prop the laptop on your knee, or just nudge it the wrong way, and your display rotates without warning. There is no built-in "lock rotation" toggle the way phones and tablets have, and the option Windows ships with is unreliable or missing entirely depending on the manufacturer.

Rotation Lock fixes that. **One click locks the orientation in laptop mode**, and it stays locked across sleep, wake, and reboot — until you click again to unlock.

## Features

- **One-click lock / unlock** from the main window or the system tray
- **Persists across sleep, wake, and reboot** — re-applies the lock automatically when the device returns
- **Auto-launch at login** — runs silently in the tray
- **Auto-lock on app start** — optional, applies the lock the moment the app launches
- **Multiple-sensor support** — pick which orientation device to lock if your laptop exposes more than one, and favorite the ones you prefer
- **Minimal footprint** — sits idle in the tray, low memory and CPU

## Installation

1. Download the latest `Rotation Lock.exe` from the [Releases](https://github.com/dylogaming/Rotation-Lock/releases) page.
2. Drop it anywhere you like (Desktop, Program Files, a custom folder).
3. Run it. Windows will prompt for elevation — admin rights are required to access the orientation sensor driver.

The app installs nothing globally. You can move or delete the `.exe` any time.

## Usage

- Click the lock icon in the main window (or left-click the tray icon) to toggle lock state.
- Right-click the tray icon for **Toggle / Show / Quit**.
- Closing the window keeps the app running in the tray. To fully exit, choose **Quit completely** in the close prompt or **Quit** from the tray.

## Building from source

Requirements:

- [Rust](https://rustup.rs/) (stable toolchain)
- Windows 10 / 11 with WebView2 (built-in on current installs)

```sh
git clone https://github.com/dylogaming/Rotation-Lock.git
cd Rotation-Lock
cargo build --release --manifest-path src-tauri/Cargo.toml
```

The compiled binary lands at `src-tauri/target/release/rotation-lock.exe`.

## License

Proprietary — see [LICENSE](LICENSE). Free for personal use; no redistribution, modification, or derivative works without written permission.

## Contact

Questions, bug reports, or licensing inquiries: **dylogamingofficial@gmail.com**

If Rotation Lock saves your sanity, you can [support development on Ko-fi](https://ko-fi.com/dylogaming) ☕
