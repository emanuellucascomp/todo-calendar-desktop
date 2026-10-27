use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_sql::{DbInstances, DbPool};
use todo_calendar_core::{
    models::*,
    repository::{Details, Repository},
    Result,
};

#[derive(Default)]
pub struct MutationGate(pub tauri::async_runtime::Mutex<()>);

pub async fn repository(app: &tauri::AppHandle) -> Result<Repository> {
    let state = app.state::<DbInstances>();
    let instances = state.0.read().await;
    let Some(DbPool::Sqlite(pool)) = instances.get(super::DATABASE_URL) else {
        return Err("O banco local ainda não está pronto.".into());
    };
    Ok(Repository {
        pool: pool.clone(),
        zone: Arc::new(crate::timezone::ZonedTime),
    })
}
pub fn changed(app: &tauri::AppHandle) {
    let _ = app.emit("calendar-changed", ());
    if let Some(runtime) = app.try_state::<crate::runtime::RuntimeState>() {
        runtime.wake.notify_one();
    }
}

#[tauri::command]
pub async fn list_occurrences(
    app: tauri::AppHandle,
    start: String,
    end: String,
) -> Result<Vec<Occurrence>> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    let from = date(&start)?;
    let to = date(&end)?;
    if to <= from || (to - from).num_days() > 400 {
        return Err("Intervalo inválido.".into());
    }
    let repo = repository(&app).await?;
    let occurrences = repo
        .occurrences(
            &(from - chrono::Duration::days(2)).to_string(),
            &(to + chrono::Duration::days(2)).to_string(),
        )
        .await?;
    let mut result = Vec::new();
    for occurrence in occurrences {
        if let Some(occurrence) = crate::timezone::display_local(occurrence)? {
            if todo_calendar_core::recurrence::overlaps(&occurrence.entry, from, to)? {
                result.push(occurrence);
            }
        }
    }
    result.sort_by(|a, b| {
        (&a.entry.start_date, &a.entry.start_time, &a.entry.id).cmp(&(
            &b.entry.start_date,
            &b.entry.start_time,
            &b.entry.id,
        ))
    });
    Ok(result)
}
#[tauri::command]
pub async fn entry_details(app: tauri::AppHandle, id: String, original: String) -> Result<Details> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    repository(&app).await?.details(&id, &original).await
}
#[tauri::command]
pub async fn save_entry(
    app: tauri::AppHandle,
    id: Option<String>,
    original: Option<String>,
    scope: Scope,
    draft: EntryDraft,
) -> Result<()> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    let repo = repository(&app).await?;
    crate::timezone::validate(&draft)?;
    if let Some(id) = id {
        repo.update(
            &id,
            original.as_deref().ok_or("Ocorrência não informada.")?,
            scope,
            &draft,
        )
        .await?;
    } else {
        repo.create(&draft).await?;
    }
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn delete_entry(
    app: tauri::AppHandle,
    id: String,
    original: String,
    scope: Scope,
) -> Result<()> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    repository(&app)
        .await?
        .remove(&id, &original, scope)
        .await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn complete_entry(
    app: tauri::AppHandle,
    id: String,
    original: String,
    completed: bool,
) -> Result<()> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    repository(&app)
        .await?
        .complete(&id, &original, completed)
        .await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn duplicate_entry(app: tauri::AppHandle, id: String, original: String) -> Result<()> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    repository(&app).await?.duplicate(&id, &original).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn save_category(
    app: tauri::AppHandle,
    id: Option<String>,
    name: String,
    color: String,
) -> Result<String> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    let id = repository(&app)
        .await?
        .save_category(id.as_deref(), &name, &color)
        .await?;
    changed(&app);
    Ok(id)
}
#[tauri::command]
pub async fn delete_category(app: tauri::AppHandle, id: String) -> Result<()> {
    let gate = app.state::<MutationGate>();
    let _lock = gate.0.lock().await;
    repository(&app).await?.delete_category(&id).await?;
    changed(&app);
    Ok(())
}
