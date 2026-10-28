use std::io::{Read, Write};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use todo_calendar_core::{backup::Backup, error, id, Result};

#[derive(serde::Serialize)]
pub struct ImportPreview {
    pub token: String,
    pub entries: usize,
    pub categories: usize,
    pub exceptions: usize,
    pub exported_at: String,
}

#[tauri::command]
pub async fn export_backup(app: tauri::AppHandle) -> Result<Option<String>> {
    let backup = {
        let gate = app.state::<crate::calendar::MutationGate>();
        let _lock = gate.0.lock().await;
        crate::calendar::repository(&app)
            .await?
            .export_backup()
            .await?
    };
    let bytes = serde_json::to_vec_pretty(&backup).map_err(error)?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err(
            "O backup excede 20 MB; exportação cancelada para manter o arquivo restaurável.".into(),
        );
    }
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("Exportar backup do Manus Calendar")
            .set_file_name(format!(
                "todo-calendar-{}.json",
                chrono::Local::now().format("%Y-%m-%d")
            ))
            .add_filter("Backup Manus Calendar", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(error)?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(error)?;
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("json"))
    {
        return Err("Salve o backup com a extensão .json.".into());
    }
    let output = path.to_string_lossy().into_owned();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let parent = path.parent().ok_or("Pasta de destino inválida.")?;
        let temporary = parent.join(format!(".todo-calendar-{}.tmp", id()));
        let write = (|| -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temporary, &path)?;
            Ok(())
        })();
        if write.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        write.map_err(error)
    })
    .await
    .map_err(error)??;
    Ok(Some(output))
}

#[tauri::command]
pub async fn preview_backup(app: tauri::AppHandle) -> Result<Option<ImportPreview>> {
    {
        let state = app.state::<crate::runtime::RuntimeState>();
        *state.pending_backup.lock().map_err(error)? = None;
    }
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("Selecionar backup para importar")
            .add_filter("Backup Manus Calendar", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(error)?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(error)?;
    let backup = tauri::async_runtime::spawn_blocking(move || -> Result<Backup> {
        if std::fs::metadata(&path).map_err(error)?.len() > 20 * 1024 * 1024 {
            return Err("O backup excede 20 MB.".into());
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(error)?
            .take(20 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(error)?;
        Backup::parse(&bytes)
    })
    .await
    .map_err(error)??;
    crate::calendar::repository(&app)
        .await?
        .validate_backup(&backup)?;
    let token = id();
    let preview = ImportPreview {
        token: token.clone(),
        entries: backup.entries.len(),
        categories: backup.categories.len(),
        exceptions: backup.exceptions.len(),
        exported_at: backup.exported_at.clone(),
    };
    *app.state::<crate::runtime::RuntimeState>()
        .pending_backup
        .lock()
        .map_err(error)? = Some((token, backup));
    Ok(Some(preview))
}

#[tauri::command]
pub async fn import_backup(app: tauri::AppHandle, token: String, confirmed: bool) -> Result<()> {
    if !confirmed {
        return Err("Confirme a substituição dos dados antes de importar.".into());
    }
    let backup = {
        let state = app.state::<crate::runtime::RuntimeState>();
        let mut pending = state.pending_backup.lock().map_err(error)?;
        if pending
            .as_ref()
            .is_none_or(|(expected, _)| *expected != token)
        {
            return Err("A seleção do backup expirou. Selecione o arquivo novamente.".into());
        }
        pending.take().unwrap().1
    };
    let gate = app.state::<crate::calendar::MutationGate>();
    let _lock = gate.0.lock().await;
    let repo = crate::calendar::repository(&app).await?;
    repo.import_backup(&backup).await?;
    let preferences = todo_calendar_core::settings::load(&repo.pool).await?;
    crate::runtime::apply(&app, &preferences);
    crate::calendar::changed(&app);
    Ok(())
}
