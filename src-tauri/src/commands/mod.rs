use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    path: String,
}

#[tauri::command]
pub fn database_info(app: tauri::AppHandle) -> Result<DatabaseInfo, String> {
    // O plugin SQL resolve URLs SQLite relativas em app_config_dir.
    let directory = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    Ok(DatabaseInfo {
        path: directory
            .join("todo-calendar.db")
            .to_string_lossy()
            .into_owned(),
    })
}
