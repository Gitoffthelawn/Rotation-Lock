// Copyright (c) 2026 DYLO Gaming LLC. All rights reserved.
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn pnputil(args: &[&str]) -> Result<(i32, String)> {
    let mut cmd = Command::new("pnputil.exe");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output()?;
    let code = out.status.code().unwrap_or(-1);
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((code, text))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SensorInfo {
    pub instance_id: String,
    pub friendly_name: String,
    pub class: String,
    pub manufacturer: String,
    pub present: bool,
    pub started: bool,
    pub priority: i32, // higher = more likely the orientation sensor
}

/// Enumerate all devices in class Sensor plus HID orientation devices.
pub fn list_sensors() -> Result<Vec<SensorInfo>> {
    let mut out = Vec::new();
    // Sensor class
    let (_, text) = pnputil(&["/enum-devices", "/class", "Sensor", "/connected"])?;
    out.extend(parse_pnputil(&text));
    // Also search by keyword across all classes (catches HID sensors)
    let (_, text2) = pnputil(&["/enum-devices", "/connected"])?;
    for s in parse_pnputil(&text2) {
        if out.iter().any(|x| x.instance_id == s.instance_id) { continue; }
        let n = s.friendly_name.to_ascii_lowercase();
        if n.contains("orientation") || n.contains("accelero") || n.contains("gyro")
            || n.contains("inclinometer") || n.contains("sensor fusion") {
            out.push(s);
        }
    }

    // Rank: hardware (ACPI / HID / PCI) above software (SWD), prefer names mentioning orientation/accel
    for s in out.iter_mut() {
        let id = s.instance_id.to_ascii_uppercase();
        let n = s.friendly_name.to_ascii_lowercase();
        let mut p = 0;
        if id.starts_with("ACPI\\") { p += 30; }
        else if id.starts_with("HID\\") { p += 20; }
        else if id.starts_with("PCI\\") { p += 10; }
        else if id.starts_with("SWD\\") { p -= 20; }
        if n.contains("orientation") { p += 15; }
        if n.contains("accelero") || n.contains("gyro") { p += 10; }
        if n.contains("fusion") { p -= 5; } // fusion hub is root, disabling it is too disruptive
        s.priority = p;
    }
    out.sort_by_key(|s| -s.priority);
    Ok(out)
}

/// Parse pnputil /enum-devices text block output.
fn parse_pnputil(text: &str) -> Vec<SensorInfo> {
    let mut result = Vec::new();
    let mut cur: Option<SensorInfo> = None;
    for raw in text.lines() {
        let line = raw.trim_end();
        if line.is_empty() {
            if let Some(s) = cur.take() {
                if !s.instance_id.is_empty() { result.push(s); }
            }
            continue;
        }
        // Lines look like: "Instance ID:                ACPI\AMDI0080\1"
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim().to_string();
            let entry = cur.get_or_insert_with(|| SensorInfo {
                instance_id: String::new(),
                friendly_name: String::new(),
                class: String::new(),
                manufacturer: String::new(),
                present: true,
                started: false,
                priority: 0,
            });
            match key.as_str() {
                "instance id" => entry.instance_id = val,
                "device description" => if entry.friendly_name.is_empty() { entry.friendly_name = val },
                "friendly name" => entry.friendly_name = val,
                "class name" => entry.class = val,
                "manufacturer name" => entry.manufacturer = val,
                "status" => {
                    let low = val.to_ascii_lowercase();
                    entry.started = low.contains("started") || low.contains("running");
                    entry.present = !low.contains("disconnected");
                }
                _ => {}
            }
        }
    }
    if let Some(s) = cur.take() {
        if !s.instance_id.is_empty() { result.push(s); }
    }
    result
}

pub fn is_present(instance_id: &str) -> bool {
    let Ok((_, text)) = pnputil(&["/enum-devices", "/instanceid", instance_id]) else { return false; };
    text.contains("Instance ID:")
}

/// Disable the sensor. Uses /remove-device which is the only path that works on
/// "critical" AMD sensors. The device reappears via scan-devices.
pub fn lock(instance_id: &str) -> Result<String> {
    // Try disable-device first (cleaner if allowed)
    let (code, out) = pnputil(&["/disable-device", instance_id])?;
    if code == 0 && !out.to_ascii_lowercase().contains("failed") {
        return Ok(format!("disable-device ok\n{out}"));
    }
    // Fall back to remove-device
    let (code2, out2) = pnputil(&["/remove-device", instance_id])?;
    if code2 == 0 {
        return Ok(format!("remove-device ok\n{out2}"));
    }
    Err(anyhow!("pnputil failed. first: {out}\nfallback: {out2}"))
}

/// Bring the sensor back. If it was removed, /scan-devices re-enumerates it.
pub fn unlock(instance_id: &str) -> Result<String> {
    // Try enable (works if it was /disable-device'd)
    let (_, out) = pnputil(&["/enable-device", instance_id])?;
    // Always follow with a scan to handle the remove-device case
    let (code, out2) = pnputil(&["/scan-devices"])?;
    if code == 0 {
        return Ok(format!("enable: {}\nscan: {}", out.trim(), out2.trim()));
    }
    Err(anyhow!("scan-devices failed: {out2}"))
}
