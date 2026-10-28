use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
use tauri::{Emitter, Manager};
use todo_calendar_core::{error, Result};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Geometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}
#[derive(Default)]
pub struct WindowSession {
    geometry: Mutex<Option<Geometry>>,
    writes: tauri::async_runtime::Mutex<()>,
    ready: AtomicBool,
    revision: AtomicU64,
}
pub async fn restore(app: &tauri::AppHandle) -> Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let position = window.outer_position().map_err(error)?;
    let size = window.inner_size().map_err(error)?;
    *app.state::<WindowSession>().geometry.lock().unwrap() = Some(Geometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: false,
    });
    let repo = crate::calendar::repository(app).await?;
    let saved: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key=?")
        .bind("window_session")
        .fetch_optional(&repo.pool)
        .await
        .map_err(error)?;
    let Some(saved) = saved else { return Ok(()) };
    let geometry: Geometry = serde_json::from_str(&saved).map_err(error)?;
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let monitors = window.available_monitors().map_err(error)?;
    if geometry.width >= 200 && geometry.height >= 200 {
        if let Some(monitor) = monitors.iter().find(|monitor| {
            visible(
                &geometry,
                monitor.position().x,
                monitor.position().y,
                monitor.size().width,
                monitor.size().height,
            )
        }) {
            window
                .set_size(tauri::PhysicalSize::new(
                    geometry.width.min(monitor.size().width),
                    geometry.height.min(monitor.size().height),
                ))
                .map_err(error)?;
            window
                .set_position(tauri::PhysicalPosition::new(geometry.x, geometry.y))
                .map_err(error)?;
        } else {
            window.center().map_err(error)?;
        }
        if geometry.maximized {
            window.maximize().map_err(error)?;
        }
        *app.state::<WindowSession>().geometry.lock().unwrap() = Some(geometry);
    }
    Ok(())
}
fn visible(g: &Geometry, x: i32, y: i32, width: u32, height: u32) -> bool {
    // A barra de título precisa permanecer acessível, inclusive em monitores à esquerda.
    i64::from(g.x) + 100 <= i64::from(x) + i64::from(width)
        && i64::from(g.x) + i64::from(g.width) >= i64::from(x) + 100
        && g.y >= y
        && i64::from(g.y) + 40 <= i64::from(y) + i64::from(height)
}
pub fn capture(window: &tauri::Window) {
    if window.label() != "main"
        || !window.is_visible().unwrap_or(false)
        || window.is_minimized().unwrap_or(true)
    {
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let state = window.state::<WindowSession>();
    let mut current = state.geometry.lock().unwrap();
    if maximized {
        if let Some(value) = current.as_mut() {
            value.maximized = true;
        }
    } else if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
        *current = Some(Geometry {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            maximized,
        });
    }
}
pub async fn persist(app: &tauri::AppHandle) -> Result<()> {
    let state = app.state::<WindowSession>();
    let _write = state.writes.lock().await;
    let geometry = state.geometry.lock().unwrap().clone();
    if let Some(geometry) = geometry {
        let repo = crate::calendar::repository(app).await?;
        sqlx::query("INSERT INTO app_settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=?")
            .bind("window_session").bind(serde_json::to_string(&geometry).map_err(error)?)
            .bind(todo_calendar_core::now()).execute(&repo.pool).await.map_err(error)?;
    }
    Ok(())
}
pub fn changed(window: &tauri::Window) {
    capture(window);
    let app = window.app_handle().clone();
    let revision = app
        .state::<WindowSession>()
        .revision
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        if app
            .state::<WindowSession>()
            .revision
            .load(Ordering::Relaxed)
            != revision
        {
            return;
        }
        if let Err(error) = persist(&app).await {
            eprintln!("Não foi possível salvar a janela: {error}");
        }
    });
}
pub fn intercept_close(window: &tauri::Window, api: &tauri::CloseRequestApi) -> bool {
    if !window
        .state::<WindowSession>()
        .ready
        .load(Ordering::Relaxed)
    {
        return false;
    }
    api.prevent_close();
    capture(window);
    let _ = window.emit("session-close-requested", false);
    true
}
#[tauri::command]
pub fn session_ready(app: tauri::AppHandle) {
    app.state::<WindowSession>()
        .ready
        .store(true, Ordering::Relaxed);
}
#[tauri::command]
pub async fn finish_session_close(app: tauri::AppHandle, quit: bool) -> Result<()> {
    persist(&app).await?;
    if quit {
        app.exit(0);
        return Ok(());
    }
    let window = app
        .get_webview_window("main")
        .ok_or("Janela não encontrada.")?;
    if app
        .state::<crate::runtime::RuntimeState>()
        .close_to_tray
        .load(Ordering::Relaxed)
    {
        window.hide().map_err(error)
    } else {
        window.destroy().map_err(error)
    }
}
pub fn quit(app: &tauri::AppHandle) {
    if app.state::<WindowSession>().ready.load(Ordering::Relaxed) {
        let _ = app.emit("session-close-requested", true);
    } else {
        app.exit(0);
    }
}
pub fn should_show<'a>(args: impl Iterator<Item = &'a str>, start_in_tray: bool) -> bool {
    !start_in_tray
        || !args
            .into_iter()
            .any(|arg| arg == "--tray" || arg == "--autostart")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_launch_opens_and_autostart_obeys_preference() {
        assert!(should_show(["app"].into_iter(), true));
        assert!(should_show(["app", "--autostart"].into_iter(), false));
        assert!(should_show(["app", "--tray"].into_iter(), false));
        assert!(!should_show(["app", "--autostart"].into_iter(), true));
        assert!(!should_show(["app", "--tray"].into_iter(), true));
    }
    #[test]
    fn restore_requires_reachable_title_bar() {
        let mut g = Geometry {
            x: -1200,
            y: 100,
            width: 1000,
            height: 700,
            maximized: false,
        };
        assert!(visible(&g, -1920, 0, 1920, 1080));
        assert!(!visible(&g, 0, 0, 1920, 1080));
        g.x = 100;
        g.y = -500;
        assert!(!visible(&g, 0, 0, 1920, 1080));
    }
}
