// Copyright (c) 2026 DYLO Gaming LLC. All rights reserved.
use anyhow::Result;
use std::mem;

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool { true }

#[cfg(windows)]
pub fn relaunch_elevated() -> Result<()> {
    use windows::core::{PCWSTR, HSTRING};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;

    let exe = std::env::current_exe()?;
    let exe_w: Vec<u16> = exe.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: HSTRING = "runas".into();

    unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe_w.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_NORMAL,
        );
    }
    Ok(())
}

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
