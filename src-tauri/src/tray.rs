use std::sync::atomic::Ordering;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use todo_calendar_core::{error, Result};

pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
pub fn install(app: &tauri::AppHandle) -> Result<()> {
    let open =
        MenuItem::with_id(app, "open", "Abrir Todo Calendar", true, None::<&str>).map_err(error)?;
    let quit = MenuItem::with_id(app, "quit", "Encerrar completamente", true, None::<&str>)
        .map_err(error)?;
    let menu = Menu::with_items(app, &[&open, &quit]).map_err(error)?;
    let icon = app
        .default_window_icon()
        .ok_or("Ícone da bandeja não encontrado.")?
        .clone();
    TrayIconBuilder::with_id("todo-calendar")
        .icon(icon)
        .tooltip("Todo Calendar")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show(tray.app_handle());
            }
        })
        .build(app)
        .map_err(error)?;
    Ok(())
}
pub fn window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    let app = window.app_handle();
    let runtime = app.state::<crate::runtime::RuntimeState>();
    match event {
        tauri::WindowEvent::CloseRequested { api, .. }
            if runtime.close_to_tray.load(Ordering::Relaxed) =>
        {
            api.prevent_close();
            let _ = window.hide();
        }
        tauri::WindowEvent::Resized(_)
            if runtime.minimize_to_tray.load(Ordering::Relaxed)
                && window.is_minimized().unwrap_or(false) =>
        {
            let _ = window.hide();
        }
        tauri::WindowEvent::Focused(true) => runtime.wake.notify_one(),
        _ => {}
    }
}
#[tauri::command]
pub fn hide_to_tray(app: tauri::AppHandle) -> Result<()> {
    app.get_webview_window("main")
        .ok_or("Janela não encontrada.")?
        .hide()
        .map_err(error)
}
