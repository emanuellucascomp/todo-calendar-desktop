use crate::{error, models::*, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Utc};
use std::collections::{HashMap, HashSet};

pub trait TimeResolver: Send + Sync {
    /// None significa horário inexistente em uma transição de verão.
    fn resolve(&self, local: NaiveDateTime, timezone: &str) -> Result<Option<DateTime<Utc>>>;
    fn local(&self, instant: DateTime<Utc>, timezone: &str) -> Result<NaiveDateTime> {
        if !["UTC", "Etc/UTC"].contains(&timezone) {
            return Err("O adaptador deve converter o instante para o fuso do registro.".into());
        }
        Ok(instant.naive_utc())
    }
}

mod rule;
pub use rule::{Frequency, Rule};

fn until_allows(
    rule: &Rule,
    candidate: NaiveDateTime,
    zone: &dyn TimeResolver,
    timezone: &str,
) -> Result<bool> {
    let Some(until) = &rule.until else {
        return Ok(true);
    };
    if until.len() == 8 {
        return Ok(candidate.date() <= NaiveDate::parse_from_str(until, "%Y%m%d").map_err(error)?);
    }
    let end = NaiveDateTime::parse_from_str(until.trim_end_matches('Z'), "%Y%m%dT%H%M%S")
        .map_err(error)?;
    if until.ends_with('Z') {
        Ok(zone
            .resolve(candidate, timezone)?
            .is_some_and(|v| v.naive_utc() <= end))
    } else {
        Ok(candidate <= end)
    }
}

pub fn starts(
    entry: &Entry,
    from: NaiveDate,
    to: NaiveDate,
    zone: &dyn TimeResolver,
) -> Result<Vec<NaiveDateTime>> {
    let anchor = local_start(entry)?;
    if entry.recurrence_rule.is_none() {
        return Ok(if anchor.date() >= from && anchor.date() < to {
            vec![anchor]
        } else {
            vec![]
        });
    }
    let rule = Rule::parse(entry.recurrence_rule.as_deref().unwrap())?;
    let cutoff = entry
        .series_end_before
        .as_deref()
        .map(key_datetime)
        .transpose()?;
    let mut index = rule.initial_period(anchor.date(), from);
    let mut count = 0usize;
    let mut result = Vec::new();
    loop {
        // Pular meses/anos sem o dia solicitado faz parte do RFC 5545.
        let dates = rule.dates(anchor.date(), index);
        index += 1;
        let period_date = match rule.frequency {
            Frequency::Daily => anchor
                .date()
                .checked_add_signed(Duration::days((index - 1) * rule.interval)),
            Frequency::Weekly => anchor
                .date()
                .checked_add_signed(Duration::weeks((index - 1) * rule.interval)),
            Frequency::Monthly => {
                let serial = anchor.year() as i64 * 12
                    + anchor.month0() as i64
                    + (index - 1) * rule.interval;
                NaiveDate::from_ymd_opt(
                    serial.div_euclid(12) as i32,
                    serial.rem_euclid(12) as u32 + 1,
                    1,
                )
            }
            Frequency::Yearly => NaiveDate::from_ymd_opt(
                (anchor.year() as i64 + (index - 1) * rule.interval) as i32,
                1,
                1,
            ),
        };
        if period_date.is_none_or(|d| d > to + Duration::days(7)) {
            break;
        }
        for day in dates {
            let candidate = day.and_time(anchor.time());
            if candidate < anchor {
                continue;
            }
            if day >= to || cutoff.is_some_and(|cutoff| candidate >= cutoff) {
                return Ok(result);
            }
            let check = if entry.start_time.is_none() {
                day.and_hms_opt(12, 0, 0).unwrap()
            } else {
                candidate
            };
            if zone.resolve(check, &entry.timezone)?.is_none() {
                continue;
            }
            if !until_allows(&rule, candidate, zone, &entry.timezone)? {
                return Ok(result);
            }
            count += 1;
            if rule.count.is_some_and(|limit| count > limit) {
                return Ok(result);
            }
            if day >= from {
                result.push(candidate);
            }
        }
        if rule.count.is_some_and(|limit| count >= limit) {
            break;
        }
    }
    Ok(result)
}

pub fn shift_entry(entry: &Entry, at: NaiveDateTime) -> Result<Entry> {
    let mut shifted = entry.clone();
    let delta = at - local_start(entry)?;
    shifted.start_date = at.date().to_string();
    shifted.start_time = entry
        .start_time
        .as_ref()
        .map(|_| at.format("%H:%M").to_string());
    if let Some(end_date) = &entry.end_date {
        let end =
            date(end_date)?.and_time(time(entry.end_time.as_deref().unwrap_or("00:00"))?) + delta;
        shifted.end_date = Some(end.date().to_string());
        shifted.end_time = entry
            .end_time
            .as_ref()
            .map(|_| end.format("%H:%M").to_string());
    }
    Ok(shifted)
}

pub fn materialize(
    entry: &Entry,
    original: &str,
    exception: Option<&Exception>,
    zone: &dyn TimeResolver,
) -> Result<Option<Occurrence>> {
    if exception.is_some_and(|v| v.exception_type == "cancelled") {
        return Ok(None);
    }
    let mut actual = shift_entry(entry, key_datetime(original)?)?;
    let mut start_utc = if actual.start_time.is_some() {
        zone.resolve(local_start(&actual)?, &actual.timezone)?
    } else {
        None
    };
    let mut end_utc = match (&actual.end_date, &actual.end_time, start_utc) {
        (Some(day), Some(clock), _) => {
            zone.resolve(date(day)?.and_time(time(clock)?), &actual.timezone)?
        }
        (_, _, Some(start)) => Some(start + Duration::minutes(30)),
        _ => None,
    };
    // DTEND/DUE define duração exata no RFC 5545; o início continua no horário local.
    if entry.start_time.is_some() {
        if let (Some(end_date), Some(end_time)) = (&entry.end_date, &entry.end_time) {
            let base_start = zone.resolve(local_start(entry)?, &entry.timezone)?;
            let base_end =
                zone.resolve(date(end_date)?.and_time(time(end_time)?), &entry.timezone)?;
            let occurrence_start = zone.resolve(local_start(&actual)?, &entry.timezone)?;
            let duration = entry
                .duration_seconds
                .map(Duration::seconds)
                .or_else(|| base_start.zip(base_end).map(|(start, end)| end - start));
            if let (Some(duration), Some(at)) = (duration, occurrence_start) {
                let local_end = zone.local(at + duration, &entry.timezone)?;
                actual.end_date = Some(local_end.date().to_string());
                actual.end_time = Some(local_end.format("%H:%M").to_string());
                end_utc = Some(at + duration);
            }
        }
    }
    if entry.recurrence_rule.is_some() {
        actual.completed = 0;
    }
    if let Some(exception) = exception {
        if let Some(json) = &exception.overrides_json {
            let patch: EntryDraft = serde_json::from_str(json).map_err(error)?;
            patch.apply(&mut actual);
            actual.recurrence_rule = entry.recurrence_rule.clone();
            start_utc = if let Some(at) = patch.start_utc {
                Some(
                    chrono::DateTime::parse_from_rfc3339(&at)
                        .map_err(error)?
                        .with_timezone(&Utc),
                )
            } else if actual.start_time.is_some() {
                zone.resolve(local_start(&actual)?, &actual.timezone)?
            } else {
                None
            };
            end_utc = if let Some(at) = patch.end_utc {
                Some(
                    chrono::DateTime::parse_from_rfc3339(&at)
                        .map_err(error)?
                        .with_timezone(&Utc),
                )
            } else if let (Some(duration), Some(start)) = (patch.duration_seconds, start_utc) {
                Some(start + Duration::seconds(duration))
            } else {
                match (&actual.end_date, &actual.end_time, start_utc) {
                    (Some(day), Some(clock), _) => {
                        zone.resolve(date(day)?.and_time(time(clock)?), &actual.timezone)?
                    }
                    (_, _, Some(start)) => Some(start + Duration::minutes(30)),
                    _ => None,
                }
            };
        }
        if let Some(completed) = exception.completed {
            actual.completed = completed;
        }
        actual.updated_at = exception.updated_at.clone();
    }
    if actual.kind == "event" {
        actual.completed = 0;
    }
    actual.duration_seconds = if actual.end_time.is_some() {
        start_utc
            .zip(end_utc)
            .map(|(start, end)| (end - start).num_seconds())
    } else {
        None
    };
    let occurrence_id = if entry.recurrence_rule.is_some() {
        format!("{}@{original}", entry.id)
    } else {
        entry.id.clone()
    };
    actual.id = occurrence_id.clone();
    Ok(Some(Occurrence {
        entry: actual,
        series_id: entry.id.clone(),
        occurrence_id,
        original_start: original.to_owned(),
        start_utc: start_utc.map(|at| at.to_rfc3339()),
        end_utc: end_utc.map(|at| at.to_rfc3339()),
    }))
}

pub fn overlaps(entry: &Entry, from: NaiveDate, to: NaiveDate) -> Result<bool> {
    let start = local_start(entry)?;
    let end = if entry.start_time.is_some() && entry.end_time.is_none() {
        start + Duration::minutes(30)
    } else {
        match &entry.end_date {
            Some(end) => date(end)?.and_time(time(entry.end_time.as_deref().unwrap_or("00:00"))?),
            None if entry.start_time.is_some() => start + Duration::minutes(30),
            None => start + Duration::days(1),
        }
    };
    let end = if end <= start && entry.end_time.is_none() {
        start + Duration::days(1)
    } else {
        end
    };
    Ok(start < to.and_hms_opt(0, 0, 0).unwrap() && end > from.and_hms_opt(0, 0, 0).unwrap())
}

pub fn expand(
    entry: &Entry,
    exceptions: &[Exception],
    from: NaiveDate,
    to: NaiveDate,
    zone: &dyn TimeResolver,
) -> Result<Vec<Occurrence>> {
    let duration = entry
        .end_date
        .as_deref()
        .map(date)
        .transpose()?
        .map(|end| (end - date(&entry.start_date).unwrap()).num_days().max(0) + 1)
        .unwrap_or(1);
    let search_from = from
        .checked_sub_signed(Duration::days(duration))
        .ok_or("Intervalo de datas inválido.")?;
    let by_key = exceptions
        .iter()
        .filter(|v| v.entry_id == entry.id)
        .map(|v| (v.original_start.as_str(), v))
        .collect::<HashMap<_, _>>();
    let mut keys = starts(entry, search_from, to, zone)?
        .iter()
        .map(|at| {
            if entry.start_time.is_some() {
                at.format("%Y-%m-%dT%H:%M").to_string()
            } else {
                at.date().to_string()
            }
        })
        .collect::<HashSet<_>>();
    // Exceções movidas para dentro do intervalo também precisam ser consultadas.
    for exception in by_key.values().filter(|v| v.overrides_json.is_some()) {
        let at = key_datetime(&exception.original_start)?;
        if starts(entry, at.date(), at.date() + Duration::days(1), zone)?.contains(&at) {
            keys.insert(exception.original_start.clone());
        }
    }
    let mut result = Vec::new();
    for key in keys {
        if let Some(occurrence) = materialize(entry, &key, by_key.get(key.as_str()).copied(), zone)?
        {
            if overlaps(&occurrence.entry, from, to)? {
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
