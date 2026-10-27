use crate::{error, id, models::*, now, recurrence, repository::Repository, Result};
use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone)]
pub struct Delivery {
    pub reminder_id: String,
    pub occurrence_key: String,
    pub scheduled_at: DateTime<Utc>,
    pub title: String,
    pub body: String,
    pub starts_at: DateTime<Utc>,
}

impl Repository {
    pub async fn due_deliveries(
        &self,
        current: DateTime<Utc>,
        since: DateTime<Utc>,
    ) -> Result<Vec<Delivery>> {
        if since > current || current - since > Duration::hours(25) {
            return Err("Intervalo de lembretes inválido.".into());
        }
        let reminders = self.reminders().await?;
        let exceptions = self.exceptions().await?;
        let mut result = Vec::new();
        for entry in self.entries().await? {
            let owned = reminders
                .iter()
                .filter(|r| r.entry_id == entry.id && r.enabled == 1)
                .collect::<Vec<_>>();
            if owned.is_empty() {
                continue;
            }
            let lead = owned
                .iter()
                .filter_map(|r| r.minutes_before)
                .max()
                .unwrap_or(0);
            let from = (since - Duration::days(2)).date_naive();
            let to = (current + Duration::minutes(lead) + Duration::days(3)).date_naive();
            let mut occurrences =
                recurrence::expand(&entry, &exceptions, from, to, self.zone.as_ref())?;
            // Um lembrete absoluto pode ocorrer anos antes da data do registro.
            // Não deve depender do horizonte calculado para antecedências relativas.
            if entry.recurrence_rule.is_none()
                && owned
                    .iter()
                    .any(|r| r.kind == "absolute" && r.exception_id.is_none())
            {
                if let Some(occurrence) = recurrence::materialize(
                    &entry,
                    &original_key(&entry),
                    None,
                    self.zone.as_ref(),
                )? {
                    if !occurrences
                        .iter()
                        .any(|v| v.occurrence_id == occurrence.occurrence_id)
                    {
                        occurrences.push(occurrence);
                    }
                }
            }
            for exception in exceptions.iter().filter(|ex| {
                ex.entry_id == entry.id
                    && ex.reminders_overridden == 1
                    && owned.iter().any(|r| {
                        r.kind == "absolute" && r.exception_id.as_deref() == Some(ex.id.as_str())
                    })
            }) {
                let at = key_datetime(&exception.original_start)?;
                if recurrence::starts(
                    &entry,
                    at.date(),
                    at.date() + Duration::days(1),
                    self.zone.as_ref(),
                )?
                .contains(&at)
                {
                    if let Some(occurrence) = recurrence::materialize(
                        &entry,
                        &exception.original_start,
                        Some(exception),
                        self.zone.as_ref(),
                    )? {
                        if !occurrences
                            .iter()
                            .any(|v| v.occurrence_id == occurrence.occurrence_id)
                        {
                            occurrences.push(occurrence);
                        }
                    }
                }
            }
            for occurrence in occurrences {
                if occurrence.entry.kind == "task" && occurrence.entry.completed == 1 {
                    continue;
                }
                let exception = exceptions.iter().find(|ex| {
                    ex.entry_id == entry.id && ex.original_start == occurrence.original_start
                });
                let exception_id = exception
                    .filter(|ex| ex.reminders_overridden == 1)
                    .map(|ex| ex.id.as_str());
                let local = date(&occurrence.entry.start_date)?.and_time(time(
                    occurrence.entry.start_time.as_deref().unwrap_or("09:00"),
                )?);
                let resolved = if let Some(at) = &occurrence.start_utc {
                    Some(
                        chrono::DateTime::parse_from_rfc3339(at)
                            .map_err(error)?
                            .with_timezone(&Utc),
                    )
                } else {
                    self.zone.resolve(local, &occurrence.entry.timezone)?
                };
                let Some(instant) = resolved else {
                    continue;
                };
                for reminder in owned
                    .iter()
                    .filter(|r| r.exception_id.as_deref() == exception_id)
                {
                    let scheduled = if reminder.kind == "relative" {
                        instant - Duration::minutes(reminder.minutes_before.unwrap_or(0))
                    } else {
                        chrono::DateTime::parse_from_rfc3339(
                            reminder
                                .scheduled_at
                                .as_deref()
                                .ok_or("Lembrete sem horário.")?,
                        )
                        .map_err(error)?
                        .with_timezone(&Utc)
                    };
                    let updated =
                        chrono::DateTime::parse_from_rfc3339(&occurrence.entry.updated_at)
                            .map_err(error)?
                            .with_timezone(&Utc);
                    let reminder_updated =
                        chrono::DateTime::parse_from_rfc3339(&reminder.updated_at)
                            .map_err(error)?
                            .with_timezone(&Utc);
                    if scheduled <= since
                        || scheduled > current
                        || scheduled < updated
                        || scheduled < reminder_updated
                    {
                        continue;
                    }
                    result.push(Delivery {
                        reminder_id: reminder.id.clone(),
                        occurrence_key: occurrence.occurrence_id.clone(),
                        scheduled_at: scheduled,
                        title: if occurrence.entry.kind == "task" {
                            "Todo Calendar · Tarefa".into()
                        } else {
                            "Todo Calendar · Evento".into()
                        },
                        body: occurrence.entry.title.clone(),
                        starts_at: instant,
                    });
                }
            }
        }
        result.sort_by_key(|delivery| delivery.scheduled_at);
        Ok(result)
    }
    pub async fn claim_delivery(&self, delivery: &Delivery) -> Result<bool> {
        let mut tx = self.pool.begin().await.map_err(error)?;
        sqlx::query("INSERT INTO notification_deliveries(id,reminder_id,occurrence_key,scheduled_at) VALUES(?,?,?,?) ON CONFLICT(reminder_id,occurrence_key) DO UPDATE SET attempts=CASE WHEN notification_deliveries.scheduled_at<>excluded.scheduled_at THEN 0 ELSE notification_deliveries.attempts END,scheduled_at=excluded.scheduled_at WHERE notification_deliveries.state IN ('pending','failed','cancelled') AND notification_deliveries.delivered_at IS NULL")
            .bind(id()).bind(&delivery.reminder_id).bind(&delivery.occurrence_key).bind(delivery.scheduled_at.to_rfc3339()).execute(&mut *tx).await.map_err(error)?;
        let changed=sqlx::query("UPDATE notification_deliveries SET state='claimed',claimed_at=?,attempts=attempts+1,error=NULL WHERE reminder_id=? AND occurrence_key=? AND state IN ('pending','failed','cancelled') AND delivered_at IS NULL AND attempts<3")
            .bind(now()).bind(&delivery.reminder_id).bind(&delivery.occurrence_key).execute(&mut *tx).await.map_err(error)?.rows_affected()==1;
        tx.commit().await.map_err(error)?;
        Ok(changed)
    }
    pub async fn finish_delivery(
        &self,
        delivery: &Delivery,
        outcome: std::result::Result<(), String>,
    ) -> Result<()> {
        let (state, delivered, error) = match outcome {
            Ok(()) => ("delivered", Some(now()), None),
            Err(error) => ("failed", None, Some(error)),
        };
        sqlx::query("UPDATE notification_deliveries SET state=?,delivered_at=?,error=? WHERE reminder_id=? AND occurrence_key=? AND state='claimed'").bind(state).bind(delivered).bind(error).bind(&delivery.reminder_id).bind(&delivery.occurrence_key).execute(&self.pool).await.map_err(crate::error)?;
        Ok(())
    }
}
