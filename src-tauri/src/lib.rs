use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;

const DEFAULT_URL: &str = "https://translate.google.com";
const STORE_PATH: &str = "settings.json";
const URL_KEY: &str = "url";

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_minimized().unwrap_or(false) {
            let _ = window.unminimize();
            let _ = window.set_focus();
        } else if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn get_stored_url(app: &tauri::AppHandle) -> String {
    if let Ok(store) = app.store(STORE_PATH) {
        if let Some(val) = store.get(URL_KEY) {
            if let Some(s) = val.as_str() {
                let s = s.trim();
                if s.starts_with("http://") || s.starts_with("https://") {
                    return s.to_string();
                }
            }
        }
    }
    DEFAULT_URL.to_string()
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use tauri_plugin_autostart::ManagerExt;
    let url = get_stored_url(&app);
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    Ok(serde_json::json!({ "url": url, "autostart": autostart }))
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL 必须以 http:// 或 https:// 开头".to_string());
    }
    let store = app.store(STORE_PATH).map_err(|e| e.to_string())?;
    store.set(URL_KEY, serde_json::json!(url));
    store.save().map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("main") {
        let js = format!("window.location.replace('{}')", url);
        let _ = window.eval(&js);
    }
    Ok(())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    if enabled {
        mgr.enable().map_err(|e| e.to_string())?;
    } else {
        mgr.disable().map_err(|e| e.to_string())?;
    }
    Ok(mgr.is_enabled().map_err(|e| e.to_string())?)
}

fn open_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("设置")
        .inner_size(420.0, 300.0)
        .center()
        .resizable(false)
        .build();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::Builder::default().build())
        .invoke_handler(tauri::generate_handler![get_settings, save_settings, set_autostart])
        .setup(|app| {
            let url = get_stored_url(app.handle());

            let _window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse().unwrap()),
            )
            .title("Google 翻译")
            .inner_size(1200.0, 800.0)
            .center()
            .visible(false)
            .initialization_script(include_str!("../../src/inject.js"))
            .build()?;

            let show_item = MenuItem::with_id(app, "show", "显示/隐藏", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &settings_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Google 翻译")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app),
                    "settings" => open_settings_window(app),
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let _ = _window.show();
            let _ = _window.set_focus();

            app.handle().global_shortcut().on_shortcut(
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyX),
                |app, _shortcut, event| {
                    if event.state == ShortcutState::Released {
                        toggle_window(app);
                    }
                },
            )?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running application");
}
