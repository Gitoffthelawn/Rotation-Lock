// Copyright (c) 2026 DYLO Gaming LLC. All rights reserved.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod elevation;
mod sensor;
mod state;
mod tasksched;

use state::AppState;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};

// Embed runtime icons at compile time.
const LOCK_GREEN_RGBA: &[u8] = include_bytes!("../icons/lock_green_64.rgba");
const LOCK_RED_RGBA:   &[u8] = include_bytes!("../icons/lock_red_64.rgba");

#[derive(Clone, serde::Serialize)]
struct StateUpdate {
    locked: bool,
    sensor_present: bool,
    message: Option<String>,
}

const RESUME_REAPPLY_AFTER: Duration = Duration::from_secs(5 * 60);
const RESUME_DEVICE_SETTLE: Duration = Duration::from_secs(5);

fn reapply_lock_if_configured(app: &AppHandle, state: &Arc<AppState>, reason: &str) {
    let Some(id) = state.config.lock().unwrap().selected_sensor.clone() else {
        return;
    };
    if !state.config.lock().unwrap().locked {
        return;
    }

    match sensor::lock(&id) {
        Ok(msg) => {
            update_tray(app, true);
            let _ = app.emit("state-changed", StateUpdate {
                locked: true,
                sensor_present: sensor::is_present(&id),
                message: Some(format!("{reason}: {msg}")),
            });
        }
        Err(err) => {
            let _ = app.emit("state-changed", StateUpdate {
                locked: true,
                sensor_present: sensor::is_present(&id),
                message: Some(format!("{reason}: failed to reapply lock: {err}")),
            });
        }
    }
}

fn start_resume_monitor(app: AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        let mut last_tick = SystemTime::now();
        loop {
            std::thread::sleep(Duration::from_secs(30));
            let now = SystemTime::now();
            let elapsed = now
                .duration_since(last_tick)
                .unwrap_or_else(|_| Duration::from_secs(0));
            last_tick = now;

            if elapsed >= RESUME_REAPPLY_AFTER {
                std::thread::sleep(RESUME_DEVICE_SETTLE);
                reapply_lock_if_configured(&app, &state, "resume");
            }
        }
    });
}

#[tauri::command]
fn cmd_list_sensors() -> Result<Vec<sensor::SensorInfo>, String> {
    sensor::list_sensors().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_config(state: tauri::State<Arc<AppState>>) -> state::Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn cmd_set_sensor(state: tauri::State<Arc<AppState>>, instance_id: String) -> Result<(), String> {
    state.config.lock().unwrap().selected_sensor = Some(instance_id);
    state.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_lock(app: AppHandle, state: tauri::State<Arc<AppState>>) -> Result<String, String> {
    let id = state.config.lock().unwrap().selected_sensor.clone()
        .ok_or("no sensor selected")?;
    let msg = sensor::lock(&id).map_err(|e| e.to_string())?;
    state.config.lock().unwrap().locked = true;
    state.save().map_err(|e| e.to_string())?;
    update_tray(&app, true);
    let _ = app.emit("state-changed", StateUpdate {
        locked: true, sensor_present: sensor::is_present(&id), message: Some(msg.clone()),
    });
    Ok(msg)
}

#[tauri::command]
fn cmd_unlock(app: AppHandle, state: tauri::State<Arc<AppState>>) -> Result<String, String> {
    let id = state.config.lock().unwrap().selected_sensor.clone()
        .ok_or("no sensor selected")?;
    let msg = sensor::unlock(&id).map_err(|e| e.to_string())?;
    state.config.lock().unwrap().locked = false;
    state.save().map_err(|e| e.to_string())?;
    update_tray(&app, false);
    let _ = app.emit("state-changed", StateUpdate {
        locked: false, sensor_present: sensor::is_present(&id), message: Some(msg.clone()),
    });
    Ok(msg)
}

#[tauri::command]
fn cmd_install_autostart() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_s = exe.to_string_lossy().into_owned();
    tasksched::install(&exe_s).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_uninstall_autostart() -> Result<(), String> {
    tasksched::uninstall().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_autostart_installed() -> bool { tasksched::is_installed() }

#[tauri::command]
fn cmd_set_start_locked(state: tauri::State<Arc<AppState>>, value: bool) -> Result<(), String> {
    state.config.lock().unwrap().start_locked = value;
    state.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_is_elevated() -> bool { elevation::is_elevated() }

#[tauri::command]
fn cmd_hide_window(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_quit_app(app: AppHandle) {
    quit_app(&app);
}

#[tauri::command]
fn cmd_open_url(url: String) -> Result<(), String> {
    // Allowlist: only http(s) URLs to avoid arbitrary command injection via shell.
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http(s) URLs are allowed".into());
    }
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn build_tray_icon(locked: bool) -> Image<'static> {
    let rgba = if locked { LOCK_RED_RGBA } else { LOCK_GREEN_RGBA };
    // Verify shape: 64 * 64 * 4 = 16384 bytes
    debug_assert_eq!(rgba.len(), 64 * 64 * 4);
    Image::new_owned(rgba.to_vec(), 64, 64)
}

fn update_tray(app: &AppHandle, locked: bool) {
    let icon = build_tray_icon(locked);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_icon(Some(icon.clone()));
        let tooltip = if locked { "Rotation Lock: LOCKED (laptop mode)" } else { "Rotation Lock: unlocked" };
        let _ = tray.set_tooltip(Some(tooltip));
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_icon(icon);
    }
}

fn toggle_from_tray(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>();
    let currently_locked = state.config.lock().unwrap().locked;
    if currently_locked {
        let _ = cmd_unlock(app.clone(), state);
    } else {
        let _ = cmd_lock(app.clone(), state);
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn quit_app(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>();
    if state.config.lock().unwrap().locked {
        let _ = cmd_unlock(app.clone(), state);
    }
    app.exit(0);
}

fn main() {
    // Self-elevate if not admin. Keep the flow simple: if not elevated, launch elevated copy and exit.
    if !elevation::is_elevated() {
        let _ = elevation::relaunch_elevated();
        return;
    }

    let started_from_tray = std::env::args().any(|a| a == "--tray");

    let app_state = Arc::new(AppState::load().expect("load config"));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            cmd_list_sensors,
            cmd_get_config,
            cmd_set_sensor,
            cmd_lock,
            cmd_unlock,
            cmd_install_autostart,
            cmd_uninstall_autostart,
            cmd_autostart_installed,
            cmd_set_start_locked,
            cmd_is_elevated,
            cmd_open_url,
            cmd_hide_window,
            cmd_quit_app,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Don't actually close — defer to the in-app modal.
                api.prevent_close();
                let _ = window.app_handle().emit("close-requested", ());
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let locked = app_state.config.lock().unwrap().locked;
            let tray_menu = Menu::with_items(&handle, &[
                &MenuItem::with_id(&handle, "toggle", "Toggle lock", true, None::<&str>)?,
                &PredefinedMenuItem::separator(&handle)?,
                &MenuItem::with_id(&handle, "show", "Show window", true, None::<&str>)?,
                &MenuItem::with_id(&handle, "quit", "Quit", true, None::<&str>)?,
            ])?;
            TrayIconBuilder::with_id("main-tray")
                .icon(build_tray_icon(locked))
                .tooltip(if locked { "Rotation Lock: LOCKED" } else { "Rotation Lock: unlocked" })
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => toggle_from_tray(app),
                    "show" => show_main_window(app),
                    "quit" => quit_app(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        toggle_from_tray(tray.app_handle());
                    }
                })
                .build(app)?;

            // Window menu bar: File / Edit / Help (Donate is rendered as an in-app button)
            let file_menu = Submenu::with_items(&handle, "&File", true, &[
                &MenuItem::with_id(&handle, "menu-hide", "Minimize to tray", true, Some("CmdOrCtrl+W"))?,
                &PredefinedMenuItem::separator(&handle)?,
                &MenuItem::with_id(&handle, "menu-quit", "Quit", true, Some("CmdOrCtrl+Q"))?,
            ])?;
            let help_menu = Submenu::with_items(&handle, "&Help", true, &[
                &MenuItem::with_id(&handle, "menu-about", "About Rotation Lock", true, None::<&str>)?,
            ])?;
            let app_menu = Menu::with_items(&handle, &[&file_menu, &help_menu])?;
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_menu(app_menu);
                win.on_menu_event(|w, event| match event.id.as_ref() {
                    "menu-hide" => { let _ = w.hide(); }
                    "menu-quit" => quit_app(w.app_handle()),
                    "menu-about" => {
                        if let Some(wv) = w.app_handle().get_webview_window("main") {
                            let _ = wv.eval("var o=document.getElementById('aboutOverlay'); if(o) o.dataset.open='true';");
                        }
                    }
                    _ => {}
                });
            }

            // Initialize window icon to match current state
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_icon(build_tray_icon(locked));
            }

            if locked {
                reapply_lock_if_configured(&handle, &app_state, "startup");
            }

            // If launched via --tray (autostart), hide main window and apply start-locked if configured.
            if started_from_tray {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
                let cfg = app_state.config.lock().unwrap().clone();
                if cfg.start_locked {
                    if let Some(id) = cfg.selected_sensor.clone() {
                        let _ = sensor::lock(&id);
                        app_state.config.lock().unwrap().locked = true;
                        let _ = app_state.save();
                        update_tray(&handle, true);
                    }
                }
            }

            start_resume_monitor(handle, app_state.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
