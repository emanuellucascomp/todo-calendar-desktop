mod categories;
mod mutations;
use crate::{
    error, id,
    models::*,
    now,
    recurrence::{self, TimeResolver},
    Result,
};
use chrono::Duration;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::sync::Arc;

#[derive(Clone)]
pub struct Repository {
    pub pool: SqlitePool,
    pub zone: Arc<dyn TimeResolver>,
}

#[derive(serde::Serialize)]
pub struct Details {
    pub occurrence: Occurrence,
    pub reminders: Vec<ReminderDraft>,
}

impl Repository {
    pub fn normalize_draft(&self, draft: &EntryDraft) -> Result<EntryDraft> {
        let mut draft = draft.clone();
        if draft.start_time.is_some() {
            if let Some(text) = &draft.recurrence_rule {
                let mut rule = recurrence::Rule::parse(text)?;
                if let Some(until) = rule.until.as_ref().filter(|value| value.len() == 8) {
                    let next = chrono::NaiveDate::parse_from_str(until, "%Y%m%d").map_err(error)?
                        + Duration::days(1);
                    let midnight = next.and_hms_opt(0, 0, 0).unwrap();
                    let mut end = None;
                    for minute in 0..=1560 {
                        if let Some(at) = self
                            .zone
                            .resolve(midnight + Duration::minutes(minute), &draft.timezone)?
                        {
                            end = Some(at - Duration::seconds(1));
                            break;
                        }
                    }
                    let end = end.ok_or("Não foi possível resolver a data final neste fuso.")?;
                    rule.until = Some(end.format("%Y%m%dT%H%M%SZ").to_string());
                    draft.recurrence_rule = Some(rule.encode());
                }
            }
        }
        Ok(draft)
    }
    pub(crate) fn validate_clock(&self, draft: &EntryDraft) -> Result<()> {
        if let (Some(start), Some(end)) = (&draft.start_utc, &draft.end_utc) {
            let start = chrono::DateTime::parse_from_rfc3339(start)
                .map_err(error)?
                .with_timezone(&chrono::Utc);
            let end = chrono::DateTime::parse_from_rfc3339(end)
                .map_err(error)?
                .with_timezone(&chrono::Utc);
            let local_start = self.zone.local(start, &draft.timezone)?;
            let local_end = self.zone.local(end, &draft.timezone)?;
            if local_start.date().to_string() != draft.start_date
                || Some(local_start.format("%H:%M").to_string()) != draft.start_time
                || Some(local_end.date().to_string()) != draft.end_date
                || Some(local_end.format("%H:%M").to_string()) != draft.end_time
            {
                return Err("Os horários não correspondem ao fuso do registro.".into());
            }
        }
        self.duration_for(draft)?;
        Ok(())
    }
    pub(crate) fn duration_for(&self, draft: &EntryDraft) -> Result<Option<i64>> {
        let entry = new_entry(draft);
        let check = if entry.start_time.is_none() {
            date(&entry.start_date)?.and_hms_opt(12, 0, 0).unwrap()
        } else {
            local_start(&entry)?
        };
        let start = self
            .zone
            .resolve(check, &entry.timezone)?
            .ok_or("O horário inicial não existe neste fuso.")?;
        if let Some(reference) = &draft.start_utc {
            if chrono::DateTime::parse_from_rfc3339(reference)
                .map_err(error)?
                .with_timezone(&chrono::Utc)
                != start
            {
                return Err(
                    "Horários iniciais ambíguos usam a primeira ocorrência do relógio.".into(),
                );
            }
        }
        if entry.start_time.is_none() || entry.end_time.is_none() {
            return Ok(None);
        }
        let end = if let Some(reference) = &draft.end_utc {
            chrono::DateTime::parse_from_rfc3339(reference)
                .map_err(error)?
                .with_timezone(&chrono::Utc)
        } else if let Some(duration) = draft.duration_seconds {
            start + Duration::seconds(duration)
        } else {
            self.zone
                .resolve(
                    local_end(&entry)?.ok_or("Término obrigatório.")?,
                    &entry.timezone,
                )?
                .ok_or("O horário final não existe neste fuso.")?
        };
        let expected = self.zone.local(end, &entry.timezone)?;
        if Some(expected.date().to_string()) != entry.end_date
            || Some(expected.format("%H:%M").to_string()) != entry.end_time
            || end <= start
        {
            return Err("O término não corresponde à duração neste fuso.".into());
        }
        let duration = (end - start).num_seconds();
        if draft
            .duration_seconds
            .is_some_and(|value| value != duration)
        {
            return Err("Duração inconsistente com os horários.".into());
        }
        Ok(Some(duration))
    }
    async fn validate_draft(&self, draft: &EntryDraft) -> Result<()> {
        draft.validate()?;
        self.validate_clock(draft)?;
        if let Some(category) = &draft.category_id {
            let exists: i64 = sqlx::query_scalar("SELECT count(*) FROM categories WHERE id=?")
                .bind(category)
                .fetch_one(&self.pool)
                .await
                .map_err(error)?;
            if exists != 1 {
                return Err("A categoria selecionada não existe mais.".into());
            }
        }
        Ok(())
    }
    pub async fn entries(&self) -> Result<Vec<Entry>> {
        sqlx::query_as("SELECT * FROM calendar_entries ORDER BY start_date,id")
            .fetch_all(&self.pool)
            .await
            .map_err(error)
    }
    pub async fn exceptions(&self) -> Result<Vec<Exception>> {
        sqlx::query_as("SELECT * FROM recurrence_exceptions")
            .fetch_all(&self.pool)
            .await
            .map_err(error)
    }
    pub async fn reminders(&self) -> Result<Vec<Reminder>> {
        sqlx::query_as("SELECT * FROM reminders")
            .fetch_all(&self.pool)
            .await
            .map_err(error)
    }
    pub async fn entry(&self, identifier: &str) -> Result<Entry> {
        sqlx::query_as("SELECT * FROM calendar_entries WHERE id=?")
            .bind(identifier)
            .fetch_optional(&self.pool)
            .await
            .map_err(error)?
            .ok_or("Registro não encontrado.".into())
    }
    pub async fn occurrences(&self, start: &str, end: &str) -> Result<Vec<Occurrence>> {
        let from = date(start)?;
        let to = date(end)?;
        if to <= from || (to - from).num_days() > 400 {
            return Err("Consulte um intervalo de até 400 dias.".into());
        }
        let exceptions = self.exceptions().await?;
        let mut result = Vec::new();
        for entry in self.entries().await? {
            result.extend(recurrence::expand(
                &entry,
                &exceptions,
                from,
                to,
                self.zone.as_ref(),
            )?);
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
    pub fn verify_original(&self, entry: &Entry, original: &str) -> Result<()> {
        let at = key_datetime(original)?;
        if !recurrence::starts(
            entry,
            at.date(),
            at.date() + Duration::days(1),
            self.zone.as_ref(),
        )?
        .contains(&at)
        {
            return Err("Esta ocorrência não pertence à série atual.".into());
        }
        Ok(())
    }
    pub async fn details(&self, identifier: &str, original: &str) -> Result<Details> {
        let entry = self.entry(identifier).await?;
        self.verify_original(&entry, original)?;
        let exception = sqlx::query_as::<_, Exception>(
            "SELECT * FROM recurrence_exceptions WHERE entry_id=? AND original_start=?",
        )
        .bind(identifier)
        .bind(original)
        .fetch_optional(&self.pool)
        .await
        .map_err(error)?;
        let occurrence =
            recurrence::materialize(&entry, original, exception.as_ref(), self.zone.as_ref())?
                .ok_or("Esta ocorrência foi excluída.")?;
        let exception_id = exception
            .as_ref()
            .filter(|e| e.reminders_overridden == 1)
            .map(|e| e.id.as_str());
        let reminders = sqlx::query_as::<_, Reminder>(
            "SELECT * FROM reminders WHERE entry_id=? AND exception_id IS ? ORDER BY created_at,id",
        )
        .bind(identifier)
        .bind(exception_id)
        .fetch_all(&self.pool)
        .await
        .map_err(error)?;
        Ok(Details {
            occurrence,
            reminders: reminders.iter().map(ReminderDraft::from).collect(),
        })
    }
}

pub fn new_entry(draft: &EntryDraft) -> Entry {
    let mut entry = Entry {
        id: id(),
        kind: String::new(),
        title: String::new(),
        description: String::new(),
        start_date: String::new(),
        start_time: None,
        end_date: None,
        end_time: None,
        all_day: 0,
        timezone: String::new(),
        category_id: None,
        priority: String::new(),
        completed: 0,
        recurrence_rule: None,
        series_parent_id: None,
        created_at: now(),
        updated_at: now(),
        series_end_before: None,
        duration_seconds: None,
        font_size: 12, bold: false, italic: false, underline: false,
    };
    draft.apply(&mut entry);
    entry
}
pub fn draft_from(entry: &Entry, reminders: Vec<ReminderDraft>) -> EntryDraft {
    EntryDraft {
        kind: entry.kind.clone(),
        title: entry.title.clone(),
        description: entry.description.clone(),
        start_date: entry.start_date.clone(),
        start_time: entry.start_time.clone(),
        end_date: entry.end_date.clone(),
        end_time: entry.end_time.clone(),
        all_day: entry.all_day == 1,
        timezone: entry.timezone.clone(),
        category_id: entry.category_id.clone(),
        priority: entry.priority.clone(),
        recurrence_rule: entry.recurrence_rule.clone(),
        reminders,
        start_utc: None,
        end_utc: None,
        duration_seconds: entry.duration_seconds,
        font_size: entry.font_size, bold: entry.bold, italic: entry.italic, underline: entry.underline,
    }
}
pub async fn write_entry(tx: &mut Transaction<'_, Sqlite>, entry: &Entry) -> Result<()> {
    sqlx::query("INSERT INTO calendar_entries(id,kind,title,description,start_date,start_time,end_date,end_time,all_day,timezone,category_id,priority,completed,recurrence_rule,series_parent_id,created_at,updated_at,series_end_before,duration_seconds,font_size,bold,italic,underline) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind,title=excluded.title,description=excluded.description,start_date=excluded.start_date,start_time=excluded.start_time,end_date=excluded.end_date,end_time=excluded.end_time,all_day=excluded.all_day,timezone=excluded.timezone,category_id=excluded.category_id,priority=excluded.priority,completed=excluded.completed,recurrence_rule=excluded.recurrence_rule,series_parent_id=excluded.series_parent_id,updated_at=excluded.updated_at,series_end_before=excluded.series_end_before,duration_seconds=excluded.duration_seconds,font_size=excluded.font_size,bold=excluded.bold,italic=excluded.italic,underline=excluded.underline")
        .bind(&entry.id).bind(&entry.kind).bind(&entry.title).bind(&entry.description).bind(&entry.start_date).bind(&entry.start_time).bind(&entry.end_date).bind(&entry.end_time).bind(entry.all_day).bind(&entry.timezone).bind(&entry.category_id).bind(&entry.priority).bind(entry.completed).bind(&entry.recurrence_rule).bind(&entry.series_parent_id).bind(&entry.created_at).bind(&entry.updated_at).bind(&entry.series_end_before).bind(entry.duration_seconds).bind(entry.font_size).bind(entry.bold).bind(entry.italic).bind(entry.underline).execute(&mut **tx).await.map_err(error)?;
    Ok(())
}
