use crate::{error, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub start_date: String,
    pub start_time: Option<String>,
    pub end_date: Option<String>,
    pub end_time: Option<String>,
    pub all_day: i64,
    pub timezone: String,
    pub category_id: Option<String>,
    pub priority: String,
    pub completed: i64,
    pub recurrence_rule: Option<String>,
    pub series_parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub series_end_before: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderDraft {
    #[serde(default)]
    pub id: Option<String>,
    pub kind: String,
    pub minutes_before: Option<i64>,
    pub scheduled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reminder {
    pub id: String,
    pub entry_id: String,
    pub exception_id: Option<String>,
    pub kind: String,
    pub minutes_before: Option<i64>,
    pub scheduled_at: Option<String>,
    pub enabled: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Reminder> for ReminderDraft {
    fn from(value: &Reminder) -> Self {
        Self {
            id: Some(value.id.clone()),
            kind: value.kind.clone(),
            minutes_before: value.minutes_before,
            scheduled_at: value.scheduled_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDraft {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub start_date: String,
    pub start_time: Option<String>,
    pub end_date: Option<String>,
    pub end_time: Option<String>,
    pub all_day: bool,
    pub timezone: String,
    pub category_id: Option<String>,
    pub priority: String,
    pub recurrence_rule: Option<String>,
    #[serde(default)]
    pub reminders: Vec<ReminderDraft>,
    #[serde(default)]
    pub start_utc: Option<String>,
    #[serde(default)]
    pub end_utc: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Exception {
    pub id: String,
    pub entry_id: String,
    pub original_start: String,
    pub exception_type: String,
    pub overrides_json: Option<String>,
    pub completed: Option<i64>,
    pub reminders_overridden: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Occurrence {
    #[serde(flatten)]
    pub entry: Entry,
    pub series_id: String,
    pub occurrence_id: String,
    pub original_start: String,
    pub start_utc: Option<String>,
    pub end_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    One,
    Future,
    Series,
}

pub fn date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(error)
}
pub fn time(value: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M").map_err(error)
}
pub fn local_start(entry: &Entry) -> Result<NaiveDateTime> {
    Ok(date(&entry.start_date)?.and_time(time(entry.start_time.as_deref().unwrap_or("00:00"))?))
}
pub fn local_end(entry: &Entry) -> Result<Option<NaiveDateTime>> {
    entry
        .end_date
        .as_deref()
        .map(|day| Ok(date(day)?.and_time(time(entry.end_time.as_deref().unwrap_or("00:00"))?)))
        .transpose()
}
pub fn original_key(entry: &Entry) -> String {
    match &entry.start_time {
        Some(t) => format!("{}T{}", entry.start_date, t),
        None => entry.start_date.clone(),
    }
}
pub fn key_datetime(value: &str) -> Result<NaiveDateTime> {
    if value.len() == 10 {
        Ok(date(value)?.and_hms_opt(0, 0, 0).unwrap())
    } else {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").map_err(error)
    }
}

impl EntryDraft {
    pub fn validate(&self) -> Result<()> {
        let reference_interval = match (&self.start_utc, &self.end_utc) {
            (None, None) => false,
            (Some(start), Some(end)) => {
                let start = chrono::DateTime::parse_from_rfc3339(start).map_err(error)?;
                let end = chrono::DateTime::parse_from_rfc3339(end).map_err(error)?;
                if self.all_day || end <= start {
                    return Err("Referência de horário inválida.".into());
                }
                true
            }
            _ => return Err("Informe as duas referências de horário.".into()),
        };
        if !["task", "event"].contains(&self.kind.as_str()) {
            return Err("Tipo de registro inválido.".into());
        }
        if self.title.trim().is_empty() || self.title.chars().count() > 300 {
            return Err("Informe um título de até 300 caracteres.".into());
        }
        if self.description.len() > 100_000 {
            return Err("Descrição muito longa.".into());
        }
        if !["low", "medium", "high"].contains(&self.priority.as_str()) {
            return Err("Prioridade inválida.".into());
        }
        if self.timezone.trim().is_empty() {
            return Err("Fuso horário obrigatório.".into());
        }
        let start = date(&self.start_date)?;
        if !(1900..=9998).contains(&chrono::Datelike::year(&start)) {
            return Err("A data deve estar entre 1900 e 9998.".into());
        }
        let end = self.end_date.as_deref().map(date).transpose()?;
        let initial_time = self.start_time.as_deref().map(time).transpose()?;
        let final_time = self.end_time.as_deref().map(time).transpose()?;
        if self.all_day && (initial_time.is_some() || final_time.is_some()) {
            return Err("Dia inteiro não deve conter horários.".into());
        }
        let exact_duration = self
            .duration_seconds
            .is_some_and(|value| value > 0 && value <= 315360000000);
        if self.duration_seconds.is_some()
            && (!exact_duration
                || self.all_day
                || initial_time.is_none()
                || final_time.is_none()
                || end.is_none())
        {
            return Err("Duração exata inválida.".into());
        }
        if end.is_some_and(|end| end < start) && !reference_interval && !exact_duration {
            return Err("O término não pode anteceder o início.".into());
        }
        if final_time.is_some() && (initial_time.is_none() || end.is_none()) {
            return Err("Informe data e horário inicial antes do término.".into());
        }
        if let (Some(a), Some(b), Some(end)) = (initial_time, final_time, end) {
            if end.and_time(b) <= start.and_time(a) && !reference_interval && !exact_duration {
                return Err("O término deve ser posterior ao início.".into());
            }
        }
        if self.kind == "event"
            && (end.is_none()
                || if self.all_day {
                    end <= Some(start)
                } else {
                    initial_time.is_none() || final_time.is_none()
                })
        {
            return Err("Eventos precisam de início e término válidos.".into());
        }
        if let Some(rule) = &self.recurrence_rule {
            crate::recurrence::Rule::parse(rule)?;
        }
        if self.reminders.len() > 12 {
            return Err("Use no máximo 12 lembretes por registro.".into());
        }
        let mut reminder_keys = std::collections::HashSet::new();
        for reminder in &self.reminders {
            match reminder.kind.as_str() {
                "relative"
                    if reminder.scheduled_at.is_none()
                        && reminder
                            .minutes_before
                            .is_some_and(|v| (0..=525_600).contains(&v)) => {}
                "absolute"
                    if reminder.minutes_before.is_none()
                        && reminder
                            .scheduled_at
                            .as_deref()
                            .is_some_and(|v| chrono::DateTime::parse_from_rfc3339(v).is_ok()) =>
                {
                    if self.recurrence_rule.is_some() {
                        return Err("Em séries, use antecedência; horários absolutos são permitidos em ocorrências individuais.".into());
                    }
                }
                _ => return Err("Lembrete inválido.".into()),
            }
            let key = if reminder.kind == "relative" {
                format!("relative:{}", reminder.minutes_before.unwrap())
            } else {
                format!(
                    "absolute:{}",
                    chrono::DateTime::parse_from_rfc3339(reminder.scheduled_at.as_deref().unwrap())
                        .map_err(error)?
                        .timestamp_millis()
                )
            };
            if !reminder_keys.insert(key) {
                return Err("Já existe um lembrete com essa antecedência ou horário.".into());
            }
        }
        Ok(())
    }
    pub fn apply(&self, entry: &mut Entry) {
        entry.kind = self.kind.clone();
        entry.title = self.title.trim().to_owned();
        entry.description = self.description.clone();
        entry.start_date = self.start_date.clone();
        entry.start_time = self.start_time.clone();
        entry.end_date = self.end_date.clone();
        entry.end_time = self.end_time.clone();
        entry.all_day = i64::from(self.all_day);
        entry.timezone = self.timezone.clone();
        entry.category_id = self.category_id.clone();
        entry.priority = self.priority.clone();
        entry.recurrence_rule = self.recurrence_rule.clone();
        entry.duration_seconds = self.duration_seconds;
        if entry.kind == "event" {
            entry.completed = 0;
        }
    }
}
