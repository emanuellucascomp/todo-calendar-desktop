use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use todo_calendar_core::{models::*, recurrence::TimeResolver, Result};

pub struct ZonedTime;
impl TimeResolver for ZonedTime {
    fn local(&self, instant: DateTime<Utc>, timezone: &str) -> Result<NaiveDateTime> {
        let zone: Tz = timezone
            .parse()
            .map_err(|_| format!("Fuso horário desconhecido: {timezone}."))?;
        Ok(instant.with_timezone(&zone).naive_local())
    }
    fn resolve(&self, local: NaiveDateTime, timezone: &str) -> Result<Option<DateTime<Utc>>> {
        let zone: Tz = timezone
            .parse()
            .map_err(|_| format!("Fuso horário desconhecido: {timezone}."))?;
        Ok(match zone.from_local_datetime(&local) {
            LocalResult::Single(value) => Some(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(a, b) => Some(a.min(b).with_timezone(&Utc)),
            LocalResult::None => None,
        })
    }
}
pub fn validate(draft: &EntryDraft) -> Result<()> {
    let entry = todo_calendar_core::repository::new_entry(draft);
    let start = local_start(&entry)?;
    let check = if draft.start_time.is_none() {
        start.date().and_hms_opt(12, 0, 0).unwrap()
    } else {
        start
    };
    if ZonedTime.resolve(check, &draft.timezone)?.is_none() {
        return Err(
            "Este horário não existe devido à mudança de horário de verão. Escolha outro horário."
                .into(),
        );
    }
    if let (Some(end), Some(time)) = (&draft.end_date, &draft.end_time) {
        if ZonedTime
            .resolve(
                date(end)?.and_time(todo_calendar_core::models::time(time)?),
                &draft.timezone,
            )?
            .is_none()
        {
            return Err("O horário final não existe neste fuso.".into());
        }
    }
    Ok(())
}
pub fn display_local(mut occurrence: Occurrence) -> Result<Option<Occurrence>> {
    if occurrence.entry.start_time.is_none() || occurrence.entry.all_day == 1 {
        return Ok(Some(occurrence));
    }
    let entry = &mut occurrence.entry;
    let start = if let Some(at) = &occurrence.start_utc {
        Some(
            chrono::DateTime::parse_from_rfc3339(at)
                .map_err(todo_calendar_core::error)?
                .with_timezone(&Utc),
        )
    } else {
        ZonedTime.resolve(local_start(entry)?, &entry.timezone)?
    };
    let Some(start) = start else {
        return Ok(None);
    };
    if let (Some(end), Some(time)) = (&entry.end_date, &entry.end_time) {
        let local = date(end)?.and_time(todo_calendar_core::models::time(time)?);
        let mut instant = if let Some(at) = &occurrence.end_utc {
            Some(
                chrono::DateTime::parse_from_rfc3339(at)
                    .map_err(todo_calendar_core::error)?
                    .with_timezone(&Utc),
            )
        } else {
            ZonedTime.resolve(local, &entry.timezone)?
        };
        // O início inexistente é pulado pelo RRULE; um término em uma lacuna avança até o primeiro minuto válido.
        for minute in 1..=180 {
            if instant.is_some() {
                break;
            }
            instant =
                ZonedTime.resolve(local + chrono::Duration::minutes(minute), &entry.timezone)?;
        }
        if let Some(instant) = instant {
            let end = instant.with_timezone(&Local);
            entry.end_date = Some(end.date_naive().to_string());
            entry.end_time = Some(end.format("%H:%M").to_string());
        }
    }
    let local = start.with_timezone(&Local);
    entry.start_date = local.date_naive().to_string();
    entry.start_time = Some(local.format("%H:%M").to_string());
    Ok(Some(occurrence))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_dst_gap_and_fold() {
        let gap = NaiveDateTime::parse_from_str("2026-03-08T02:30", "%Y-%m-%dT%H:%M").unwrap();
        assert!(ZonedTime
            .resolve(gap, "America/New_York")
            .unwrap()
            .is_none());
        let fold = NaiveDateTime::parse_from_str("2026-11-01T01:30", "%Y-%m-%dT%H:%M").unwrap();
        assert_eq!(
            ZonedTime
                .resolve(fold, "America/New_York")
                .unwrap()
                .unwrap()
                .to_rfc3339(),
            "2026-11-01T05:30:00+00:00"
        );
    }
    #[test]
    fn timezone_change_keeps_distinct_instants() {
        let at = date("2026-11-02").unwrap().and_hms_opt(9, 0, 0).unwrap();
        assert_ne!(
            ZonedTime.resolve(at, "America/Sao_Paulo").unwrap(),
            ZonedTime.resolve(at, "Europe/Lisbon").unwrap()
        );
    }
}
