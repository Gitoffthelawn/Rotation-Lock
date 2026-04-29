// Copyright (c) 2026 DYLO Gaming LLC. All rights reserved.
use anyhow::{anyhow, Result};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const TASK_NAME: &str = "Rotation Lock Autostart";
const OLD_TASK_NAME: &str = "RotationLockAutostart";

pub fn is_installed() -> bool {
    task_exists(TASK_NAME) || task_exists(OLD_TASK_NAME)
}

fn task_exists(name: &str) -> bool {
    let mut cmd = Command::new("schtasks.exe");
    cmd.args(["/Query", "/TN", name]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn install(exe: &str) -> Result<()> {
    let user = format!("{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default());

    let xml = format!(r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Auto-start Rotation Lock at logon (elevated).</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>--tray</Arguments>
    </Exec>
  </Actions>
</Task>"#);

    // Write to temp file with UTF-16 LE BOM
    let temp = std::env::temp_dir().join("rotation-lock-task.xml");
    write_utf16(&temp, &xml)?;

    let mut cmd = Command::new("schtasks.exe");
    cmd.args(["/Create", "/TN", TASK_NAME, "/XML",
        temp.to_str().ok_or_else(|| anyhow!("bad path"))?, "/F"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output()?;
    let _ = std::fs::remove_file(&temp);
    if !out.status.success() {
        return Err(anyhow!("schtasks /Create failed: {}",
            String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

pub fn uninstall() -> Result<()> {
    uninstall_task(TASK_NAME)?;
    uninstall_task(OLD_TASK_NAME)?;
    Ok(())
}

fn uninstall_task(name: &str) -> Result<()> {
    if !task_exists(name) { return Ok(()); }
    let mut cmd = Command::new("schtasks.exe");
    cmd.args(["/Delete", "/TN", name, "/F"]);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(anyhow!("schtasks /Delete failed: {}",
            String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

fn write_utf16(path: &std::path::Path, text: &str) -> Result<()> {
    let mut bytes: Vec<u8> = vec![0xFF, 0xFE]; // UTF-16 LE BOM
    for u in text.encode_utf16() {
        bytes.push((u & 0xFF) as u8);
        bytes.push((u >> 8) as u8);
    }
    std::fs::write(path, bytes)?;
    Ok(())
}
