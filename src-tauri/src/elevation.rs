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

    // Forward the original CLI args (skipping argv[0]) so flags like --tray
    // survive the UAC self-elevation hop. Quote each arg defensively.
    let args: Vec<String> = std::env::args().skip(1)
        .map(|a| if a.contains(' ') { format!("\"{}\"", a) } else { a })
        .collect();
    let args_joined = args.join(" ");
    let args_w: Vec<u16> = args_joined.encode_utf16().chain(std::iter::once(0)).collect();
    let params_ptr = if args.is_empty() { PCWSTR::null() } else { PCWSTR(args_w.as_ptr()) };

    unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe_w.as_ptr()),
            params_ptr,
            PCWSTR::null(),
            SW_NORMAL,
        );
    }
    Ok(())
}

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
