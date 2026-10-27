use crate::{error, Result};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}
#[derive(Clone, Debug)]
pub struct Rule {
    pub frequency: Frequency,
    pub interval: i64,
    pub weekdays: Vec<u32>,
    pub count: Option<usize>,
    pub until: Option<String>,
    pub week_start: u32,
}

impl Rule {
    pub fn parse(input: &str) -> Result<Self> {
        let mut parts = HashMap::new();
        for part in input.strip_prefix("RRULE:").unwrap_or(input).split(';') {
            let (key, value) = part.split_once('=').ok_or("Regra de repetição inválida.")?;
            if parts.insert(key, value).is_some() {
                return Err("Campo duplicado na repetição.".into());
            }
            if !["FREQ", "INTERVAL", "BYDAY", "COUNT", "UNTIL", "WKST"].contains(&key) {
                return Err(format!("Campo de repetição não suportado: {key}."));
            }
        }
        let frequency = match parts.get("FREQ").copied() {
            Some("DAILY") => Frequency::Daily,
            Some("WEEKLY") => Frequency::Weekly,
            Some("MONTHLY") => Frequency::Monthly,
            Some("YEARLY") => Frequency::Yearly,
            _ => return Err("Frequência de repetição inválida.".into()),
        };
        let interval = parts
            .get("INTERVAL")
            .unwrap_or(&"1")
            .parse::<i64>()
            .map_err(error)?;
        if !(1..=999).contains(&interval) {
            return Err("O intervalo deve estar entre 1 e 999.".into());
        }
        let count = parts
            .get("COUNT")
            .map(|v| v.parse::<usize>().map_err(error))
            .transpose()?;
        if count.is_some_and(|v| v == 0 || v > 100_000) {
            return Err("Use entre 1 e 100.000 ocorrências.".into());
        }
        let until = parts.get("UNTIL").map(|v| v.to_string());
        if count.is_some() && until.is_some() {
            return Err("Escolha data final ou quantidade, não ambas.".into());
        }
        if let Some(value) = &until {
            if NaiveDate::parse_from_str(value, "%Y%m%d").is_err()
                && NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y%m%dT%H%M%S")
                    .is_err()
            {
                return Err("Data final da repetição inválida.".into());
            }
        }
        let names = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
        let mut weekdays = Vec::new();
        if let Some(value) = parts.get("BYDAY") {
            for name in value.split(',') {
                weekdays.push(
                    names
                        .iter()
                        .position(|n| *n == name)
                        .ok_or("Dia da semana inválido.")? as u32,
                );
            }
            weekdays.sort_unstable();
            weekdays.dedup();
        }
        let week_start = names
            .iter()
            .position(|n| Some(n) == parts.get("WKST"))
            .unwrap_or(0) as u32;
        if parts.contains_key("WKST") && !names.contains(&parts["WKST"]) {
            return Err("Início de semana inválido.".into());
        }
        Ok(Self {
            frequency,
            interval,
            weekdays,
            count,
            until,
            week_start,
        })
    }
    pub fn encode(&self) -> String {
        let freq = match self.frequency {
            Frequency::Daily => "DAILY",
            Frequency::Weekly => "WEEKLY",
            Frequency::Monthly => "MONTHLY",
            Frequency::Yearly => "YEARLY",
        };
        let mut parts = vec![
            format!("FREQ={freq}"),
            format!("INTERVAL={}", self.interval),
        ];
        let names = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
        if !self.weekdays.is_empty() {
            parts.push(format!(
                "BYDAY={}",
                self.weekdays
                    .iter()
                    .map(|d| names[*d as usize])
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if self.week_start != 0 {
            parts.push(format!("WKST={}", names[self.week_start as usize]));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={count}"));
        }
        if let Some(until) = &self.until {
            parts.push(format!("UNTIL={until}"));
        }
        parts.join(";")
    }
    pub(super) fn dates(&self, anchor: NaiveDate, index: i64) -> Vec<NaiveDate> {
        let step = index * self.interval;
        match self.frequency {
            Frequency::Daily => anchor
                .checked_add_signed(Duration::days(step))
                .filter(|d| {
                    self.weekdays.is_empty()
                        || self.weekdays.contains(&d.weekday().num_days_from_monday())
                })
                .into_iter()
                .collect(),
            Frequency::Weekly => {
                let offset = (anchor.weekday().num_days_from_monday() + 7 - self.week_start) % 7;
                let Some(base) = anchor
                    .checked_sub_signed(Duration::days(offset as i64))
                    .and_then(|d| d.checked_add_signed(Duration::weeks(step)))
                else {
                    return vec![];
                };
                let days = if self.weekdays.is_empty() {
                    vec![anchor.weekday().num_days_from_monday()]
                } else {
                    self.weekdays.clone()
                };
                let mut dates = days
                    .iter()
                    .filter_map(|day| {
                        base.checked_add_signed(Duration::days(
                            ((day + 7 - self.week_start) % 7) as i64,
                        ))
                    })
                    .collect::<Vec<_>>();
                dates.sort();
                dates
            }
            Frequency::Monthly => {
                let serial = anchor.year() as i64 * 12 + anchor.month0() as i64 + step;
                let year = serial.div_euclid(12) as i32;
                let month = serial.rem_euclid(12) as u32 + 1;
                if self.weekdays.is_empty() {
                    NaiveDate::from_ymd_opt(year, month, anchor.day())
                        .into_iter()
                        .collect()
                } else {
                    (1..=31)
                        .filter_map(|day| NaiveDate::from_ymd_opt(year, month, day))
                        .filter(|d| self.weekdays.contains(&d.weekday().num_days_from_monday()))
                        .collect()
                }
            }
            Frequency::Yearly => {
                let year = anchor.year() as i64 + step;
                if !(1900..=9999).contains(&year) {
                    return vec![];
                }
                if self.weekdays.is_empty() {
                    NaiveDate::from_ymd_opt(year as i32, anchor.month(), anchor.day())
                        .into_iter()
                        .collect()
                } else {
                    (1..=366)
                        .filter_map(|day| NaiveDate::from_yo_opt(year as i32, day))
                        .filter(|d| self.weekdays.contains(&d.weekday().num_days_from_monday()))
                        .collect()
                }
            }
        }
    }
    pub(super) fn initial_period(&self, anchor: NaiveDate, from: NaiveDate) -> i64 {
        if self.count.is_some() {
            return 0;
        }
        let periods = match self.frequency {
            Frequency::Daily => (from - anchor).num_days(),
            Frequency::Weekly => (from - anchor).num_days() / 7,
            Frequency::Monthly => {
                (from.year() - anchor.year()) as i64 * 12 + from.month0() as i64
                    - anchor.month0() as i64
            }
            Frequency::Yearly => (from.year() - anchor.year()) as i64,
        };
        (periods / self.interval - 1).max(0)
    }
}
