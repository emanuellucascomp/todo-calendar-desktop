use chrono::{Duration, Utc};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;
use todo_calendar_core::{
    error,
    settings::{self, Preferences},
    Result,
};

#[derive(Default, Clone, serde::Serialize)]
pub struct RuntimeStatus {
    pub last_check: Option<String>,
    pub last_error: Option<String>,
}
#[derive(Default)]
pub struct RuntimeState {
    pub wake: tokio::sync::Notify,
    pub close_to_tray: AtomicBool,
    pub minimize_to_tray: AtomicBool,
    pub start_in_tray: AtomicBool,
    pub status: Mutex<RuntimeStatus>,
    pub pending_backup: Mutex<Option<(String, todo_calendar_core::backup::Backup)>>,
}
pub fn apply(app: &tauri::AppHandle, preferences: &Preferences) {
    let state = app.state::<RuntimeState>();
    state.start_in_tray.store(preferences.start_in_tray, Ordering::Relaxed);
    state
        .close_to_tray
        .store(preferences.close_behavior == "tray", Ordering::Relaxed);
    state
        .minimize_to_tray
        .store(preferences.minimize_to_tray, Ordering::Relaxed);
    state.wake.notify_one();
}
pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let result = tick(&app).await;
            if let Ok(mut status) = app.state::<RuntimeState>().status.lock() {
                status.last_check = Some(todo_calendar_core::now());
                status.last_error = result.err();
            }
            let state = app.state::<RuntimeState>();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(15), state.wake.notified())
                .await;
        }
    });
}
async fn tick(app: &tauri::AppHandle) -> Result<()> {
    let gate = app.state::<crate::calendar::MutationGate>();
    let _lock = gate.0.lock().await;
    let repo = crate::calendar::repository(app).await?;
    let preferences = settings::load(&repo.pool).await?;
    let mut notification_error = None;
    if preferences.notifications_enabled {
        let current = Utc::now();
        for delivery in repo
            .due_deliveries(current, current - Duration::hours(24))
            .await?
        {
            if repo.claim_delivery(&delivery).await? {
                let outcome = app
                    .notification()
                    .builder()
                    .title(&delivery.title)
                    .body(format!(
                        "{} · {}",
                        delivery.body,
                        delivery
                            .starts_at
                            .with_timezone(&chrono::Local)
                            .format("%d/%m/%Y %H:%M")
                    ))
                    .show()
                    .map_err(error);
                if let Err(error) = &outcome {
                    notification_error = Some(error.clone());
                }
                repo.finish_delivery(&delivery, outcome).await?;
            }
        }
    }
    sqlx::query("INSERT INTO app_settings(key,value) VALUES('scheduler_last_check',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(serde_json::to_string(&todo_calendar_core::now()).map_err(error)?).execute(&repo.pool).await.map_err(error)?;
    if let Some(error) = notification_error {
        return Err(error);
    }
    Ok(())
}
#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<Preferences> {
    let repo = crate::calendar::repository(&app).await?;
    let mut preferences = settings::load(&repo.pool).await?;
    preferences.autostart = app.autolaunch().is_enabled().map_err(error)?;
    Ok(preferences)
}
#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, preferences: Preferences) -> Result<()> {
    preferences.validate()?;
    let gate = app.state::<crate::calendar::MutationGate>();
    let _lock = gate.0.lock().await;
    let repo = crate::calendar::repository(&app).await?;
    let previous = app.autolaunch().is_enabled().map_err(error)?;
    if previous != preferences.autostart {
        if preferences.autostart {
            app.autolaunch().enable().map_err(error)?;
        } else {
            app.autolaunch().disable().map_err(error)?;
        }
    }
    if let Err(failure) = settings::save(&repo.pool, &preferences).await {
        let rollback = if previous {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };
        if let Err(rollback) = rollback {
            return Err(format!(
                "{failure}. Não foi possível restaurar a inicialização automática: {rollback}"
            ));
        }
        return Err(failure);
    }
    apply(&app, &preferences);
    crate::calendar::changed(&app);
    Ok(())
}
#[tauri::command]
pub fn test_notification(app: tauri::AppHandle) -> Result<()> {
    app.notification()
        .builder()
        .title("Manus Calendar")
        .body("Tudo pronto! Seus lembretes aparecerão aqui enquanto o aplicativo estiver ativo.")
        .show()
        .map_err(error)
}
#[tauri::command]
pub async fn runtime_status(app: tauri::AppHandle) -> Result<RuntimeStatus> {
    let mut status = app
        .state::<RuntimeState>()
        .status
        .lock()
        .map_err(error)?
        .clone();
    if status.last_error.is_none() {
        let repo = crate::calendar::repository(&app).await?;
        status.last_error=sqlx::query_scalar::<_,Option<String>>("SELECT error FROM notification_deliveries WHERE state='failed' ORDER BY claimed_at DESC LIMIT 1").fetch_optional(&repo.pool).await.map_err(error)?.flatten();
    }
    Ok(status)
}
