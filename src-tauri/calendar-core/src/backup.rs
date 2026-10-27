use crate::{
    error,
    models::*,
    now,
    repository::{draft_from, write_entry, Repository},
    settings::Preferences,
    Result,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeliveryRow {
    pub id: String,
    pub reminder_id: String,
    pub occurrence_key: String,
    pub scheduled_at: String,
    pub delivered_at: Option<String>,
    pub state: String,
    pub claimed_at: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub attempts: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub format: String,
    pub version: u32,
    pub schema_version: u32,
    pub exported_at: String,
    pub categories: Vec<Category>,
    pub entries: Vec<Entry>,
    pub exceptions: Vec<Exception>,
    pub reminders: Vec<Reminder>,
    pub deliveries: Vec<DeliveryRow>,
    pub settings: Vec<Setting>,
}
impl Backup {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 20 * 1024 * 1024 {
            return Err("O backup excede o limite de 20 MB.".into());
        }
        let backup: Self =
            serde_json::from_slice(bytes).map_err(|_| "Arquivo de backup inválido.".to_string())?;
        backup.validate()?;
        Ok(backup)
    }
    pub fn validate(&self) -> Result<()> {
        if self.format != "todo-calendar" || self.version != 1 || self.schema_version != 4 {
            return Err("Formato ou versão de backup incompatível.".into());
        }
        if self.entries.len() > 100_000
            || self.exceptions.len() > 100_000
            || self.reminders.len() > 100_000
            || self.deliveries.len() > 200_000
            || self.categories.len() > 10_000
            || self.settings.len() > 1000
        {
            return Err("O backup excede os limites de registros.".into());
        }
        fn unique<'a>(ids: impl Iterator<Item = &'a str>) -> Result<HashSet<&'a str>> {
            let mut set = HashSet::new();
            for id in ids {
                if id.is_empty() || id.len() > 200 || !set.insert(id) {
                    return Err("Backup contém identificadores inválidos ou duplicados.".into());
                }
            }
            Ok(set)
        }
        let categories = unique(self.categories.iter().map(|c| c.id.as_str()))?;
        let entries = unique(self.entries.iter().map(|e| e.id.as_str()))?;
        let exception_ids = unique(self.exceptions.iter().map(|e| e.id.as_str()))?;
        let reminders = unique(self.reminders.iter().map(|r| r.id.as_str()))?;
        let entry_by_id = self
            .entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry))
            .collect::<HashMap<_, _>>();
        let exception_by_id = self
            .exceptions
            .iter()
            .map(|exception| (exception.id.as_str(), exception))
            .collect::<HashMap<_, _>>();
        let mut root_reminders: HashMap<&str, Vec<ReminderDraft>> = HashMap::new();
        for reminder in &self.reminders {
            if reminder.exception_id.is_none() {
                root_reminders
                    .entry(&reminder.entry_id)
                    .or_default()
                    .push(ReminderDraft::from(reminder));
            }
        }
        unique(self.deliveries.iter().map(|d| d.id.as_str()))?;
        unique(self.settings.iter().map(|s| s.key.as_str()))?;
        let mut occurrence_keys = HashSet::new();
        let mut delivery_keys = HashSet::new();
        for category in &self.categories {
            if category.name.trim().is_empty()
                || category.name.chars().count() > 80
                || category.color.len() != 7
                || !category.color.starts_with('#')
                || !category.color[1..].chars().all(|c| c.is_ascii_hexdigit())
            {
                return Err("Categoria inválida no backup.".into());
            }
        }
        for entry in &self.entries {
            let draft = draft_from(
                entry,
                root_reminders
                    .get(entry.id.as_str())
                    .cloned()
                    .unwrap_or_default(),
            );
            draft.validate()?;
            if entry
                .category_id
                .as_deref()
                .is_some_and(|id| !categories.contains(id))
                || entry
                    .series_parent_id
                    .as_deref()
                    .is_some_and(|id| !entries.contains(id) || id == entry.id)
            {
                return Err("Referência inválida em um registro do backup.".into());
            }
            if ![0, 1].contains(&entry.completed) || (entry.kind == "event" && entry.completed != 0)
            {
                return Err("Conclusão inválida no backup.".into());
            }
            if let Some(cutoff) = &entry.series_end_before {
                key_datetime(cutoff)?;
            }
            chrono::DateTime::parse_from_rfc3339(&entry.updated_at).map_err(error)?;
            chrono::DateTime::parse_from_rfc3339(&entry.created_at).map_err(error)?;
        }
        for ex in &self.exceptions {
            if !entries.contains(ex.entry_id.as_str())
                || !occurrence_keys.insert((&ex.entry_id, &ex.original_start))
                || !["override", "cancelled", "completion"].contains(&ex.exception_type.as_str())
                || !ex.completed.is_none_or(|v| [0, 1].contains(&v))
                || ![0, 1].contains(&ex.reminders_overridden)
            {
                return Err("Exceção inválida no backup.".into());
            }
            key_datetime(&ex.original_start)?;
            if ex.exception_type == "completion" && ex.completed.is_none() {
                return Err("Conclusão de ocorrência ausente.".into());
            }
            if ex.exception_type == "override" && ex.overrides_json.is_none() {
                return Err("Dados da ocorrência ausentes.".into());
            }
            if let Some(json) = &ex.overrides_json {
                let draft: EntryDraft = serde_json::from_str(json).map_err(error)?;
                draft.validate()?;
                if draft
                    .category_id
                    .as_deref()
                    .is_some_and(|id| !categories.contains(id))
                {
                    return Err("Categoria da exceção inexistente.".into());
                }
            }
        }
        for reminder in &self.reminders {
            if !entries.contains(reminder.entry_id.as_str()) || ![0, 1].contains(&reminder.enabled)
            {
                return Err("Lembrete inválido no backup.".into());
            }
            if let Some(exception) = &reminder.exception_id {
                if !exception_ids.contains(exception.as_str())
                    || !exception_by_id
                        .get(exception.as_str())
                        .is_some_and(|ex| ex.entry_id == reminder.entry_id)
                {
                    return Err("Exceção de lembrete inválida.".into());
                }
            }
            let mut check = draft_from(
                entry_by_id[reminder.entry_id.as_str()],
                vec![ReminderDraft::from(reminder)],
            );
            if reminder.exception_id.is_some() {
                check.recurrence_rule = None;
            }
            check.validate()?;
        }
        for delivery in &self.deliveries {
            if !reminders.contains(delivery.reminder_id.as_str())
                || !delivery_keys.insert((&delivery.reminder_id, &delivery.occurrence_key))
                || !["pending", "claimed", "delivered", "cancelled", "failed"]
                    .contains(&delivery.state.as_str())
                || delivery.attempts < 0
            {
                return Err("Histórico de entrega inválido.".into());
            }
            chrono::DateTime::parse_from_rfc3339(&delivery.scheduled_at).map_err(error)?;
            if let Some(at) = &delivery.delivered_at {
                chrono::DateTime::parse_from_rfc3339(at).map_err(error)?;
            }
        }
        let mut settings = serde_json::to_value(Preferences::default()).map_err(error)?;
        for row in &self.settings {
            let value: serde_json::Value = serde_json::from_str(&row.value).map_err(error)?;
            if settings.get(&row.key).is_some() {
                settings[&row.key] = value.clone();
            }
            if row.key == "last_view"
                && !value
                    .as_str()
                    .is_some_and(|v| ["month", "week", "week-grid", "year"].contains(&v))
            {
                return Err("Visualização inválida no backup.".into());
            }
        }
        serde_json::from_value::<Preferences>(settings)
            .map_err(error)?
            .validate()?;
        Ok(())
    }
}

impl Repository {
    pub async fn export_backup(&self) -> Result<Backup> {
        let mut tx = self.pool.begin().await.map_err(error)?;
        let categories = sqlx::query_as("SELECT * FROM categories")
            .fetch_all(&mut *tx)
            .await
            .map_err(error)?;
        let entries = sqlx::query_as("SELECT * FROM calendar_entries")
            .fetch_all(&mut *tx)
            .await
            .map_err(error)?;
        let exceptions = sqlx::query_as("SELECT * FROM recurrence_exceptions")
            .fetch_all(&mut *tx)
            .await
            .map_err(error)?;
        let reminders = sqlx::query_as("SELECT * FROM reminders")
            .fetch_all(&mut *tx)
            .await
            .map_err(error)?;
        let deliveries = sqlx::query_as("SELECT * FROM notification_deliveries")
            .fetch_all(&mut *tx)
            .await
            .map_err(error)?;
        let settings = sqlx::query_as(
            "SELECT * FROM app_settings WHERE key NOT IN ('autostart','scheduler_last_check')",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(error)?;
        tx.commit().await.map_err(error)?;
        Ok(Backup {
            format: "todo-calendar".into(),
            version: 1,
            schema_version: 4,
            exported_at: now(),
            categories,
            entries,
            exceptions,
            reminders,
            deliveries,
            settings,
        })
    }
    pub fn validate_backup(&self, backup: &Backup) -> Result<()> {
        backup.validate()?;
        for entry in &backup.entries {
            self.validate_clock(&draft_from(entry, vec![]))?;
            self.zone.resolve(local_start(entry)?, &entry.timezone)?;
        }
        for ex in &backup.exceptions {
            if let Some(json) = &ex.overrides_json {
                let draft: EntryDraft = serde_json::from_str(json).map_err(error)?;
                self.validate_clock(&draft)?;
                let entry = crate::repository::new_entry(&draft);
                self.zone.resolve(local_start(&entry)?, &entry.timezone)?;
            }
        }
        Ok(())
    }
    pub async fn import_backup(&self, backup: &Backup) -> Result<()> {
        self.validate_backup(backup)?;
        let mut tx = self.pool.begin().await.map_err(error)?;
        sqlx::query("PRAGMA defer_foreign_keys=ON")
            .execute(&mut *tx)
            .await
            .map_err(error)?;
        let previous = sqlx::query_as::<_, DeliveryRow>(
            "SELECT * FROM notification_deliveries WHERE state IN ('delivered','claimed')",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(error)?;
        for statement in [
            "DELETE FROM notification_deliveries",
            "DELETE FROM reminders",
            "DELETE FROM recurrence_exceptions",
            "DELETE FROM calendar_entries",
            "DELETE FROM categories",
            "DELETE FROM app_settings WHERE key NOT IN ('autostart','scheduler_last_check')",
        ] {
            sqlx::query(statement)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        }
        for category in &backup.categories {
            sqlx::query(
                "INSERT INTO categories(id,name,color,created_at,updated_at) VALUES(?,?,?,?,?)",
            )
            .bind(&category.id)
            .bind(&category.name)
            .bind(&category.color)
            .bind(&category.created_at)
            .bind(&category.updated_at)
            .execute(&mut *tx)
            .await
            .map_err(error)?;
        }
        for entry in &backup.entries {
            write_entry(&mut tx, entry).await?;
        }
        for ex in &backup.exceptions {
            sqlx::query("INSERT INTO recurrence_exceptions(id,entry_id,original_start,exception_type,overrides_json,completed,reminders_overridden,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)").bind(&ex.id).bind(&ex.entry_id).bind(&ex.original_start).bind(&ex.exception_type).bind(&ex.overrides_json).bind(ex.completed).bind(ex.reminders_overridden).bind(&ex.created_at).bind(&ex.updated_at).execute(&mut *tx).await.map_err(error)?;
        }
        for r in &backup.reminders {
            sqlx::query("INSERT INTO reminders(id,entry_id,exception_id,kind,minutes_before,scheduled_at,enabled,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)").bind(&r.id).bind(&r.entry_id).bind(&r.exception_id).bind(&r.kind).bind(r.minutes_before).bind(&r.scheduled_at).bind(r.enabled).bind(&r.created_at).bind(&r.updated_at).execute(&mut *tx).await.map_err(error)?;
        }
        let reminder_ids = backup
            .reminders
            .iter()
            .map(|r| r.id.as_str())
            .collect::<HashSet<_>>();
        let mut deliveries = backup
            .deliveries
            .iter()
            .cloned()
            .map(|d| ((d.reminder_id.clone(), d.occurrence_key.clone()), d))
            .collect::<HashMap<_, _>>();
        for previous in previous {
            if reminder_ids.contains(previous.reminder_id.as_str()) {
                deliveries.insert(
                    (
                        previous.reminder_id.clone(),
                        previous.occurrence_key.clone(),
                    ),
                    previous,
                );
            }
        }
        for d in deliveries.values() {
            sqlx::query("INSERT INTO notification_deliveries(id,reminder_id,occurrence_key,scheduled_at,delivered_at,state,claimed_at,error,created_at,attempts) VALUES(?,?,?,?,?,?,?,?,?,?)").bind(&d.id).bind(&d.reminder_id).bind(&d.occurrence_key).bind(&d.scheduled_at).bind(&d.delivered_at).bind(&d.state).bind(&d.claimed_at).bind(&d.error).bind(&d.created_at).bind(d.attempts).execute(&mut *tx).await.map_err(error)?;
        }
        for setting in &backup.settings {
            if ["autostart", "scheduler_last_check"].contains(&setting.key.as_str()) {
                continue;
            }
            sqlx::query("INSERT INTO app_settings(key,value,updated_at) VALUES(?,?,?)")
                .bind(&setting.key)
                .bind(&setting.value)
                .bind(&setting.updated_at)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        }
        tx.commit().await.map_err(error)
    }
}
