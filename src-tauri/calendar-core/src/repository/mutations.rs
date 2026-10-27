use super::*;
use std::collections::HashSet;

impl Repository {
    pub async fn create(&self, draft: &EntryDraft) -> Result<Entry> {
        let normalized = self.normalize_draft(draft)?;
        let draft = &normalized;
        self.validate_draft(draft).await?;
        let mut entry = new_entry(draft);
        entry.duration_seconds = self.duration_for(draft)?;
        let mut tx = self.pool.begin().await.map_err(error)?;
        write_entry(&mut tx, &entry).await?;
        sync_reminders(&mut tx, &entry.id, None, &draft.reminders).await?;
        tx.commit().await.map_err(error)?;
        Ok(entry)
    }
    pub async fn update(
        &self,
        identifier: &str,
        original: &str,
        scope: Scope,
        draft: &EntryDraft,
    ) -> Result<()> {
        let normalized = self.normalize_draft(draft)?;
        let draft = &normalized;
        self.validate_draft(draft).await?;
        let root = self.entry(identifier).await?;
        self.verify_original(&root, original)?;
        let details = self.details(identifier, original).await?;
        let recurring = root.recurrence_rule.is_some();
        let at = key_datetime(original)?;
        let before = if recurring {
            recurrence::starts(
                &root,
                date(&root.start_date)?,
                at.date() + Duration::days(1),
                self.zone.as_ref(),
            )?
            .iter()
            .filter(|value| **value < at)
            .count()
        } else {
            0
        };
        let mut tx = self.pool.begin().await.map_err(error)?;
        if recurring && scope == Scope::One {
            let exception_id = ensure_exception(&mut tx, identifier, original).await?;
            let mut patch = draft.clone();
            patch.recurrence_rule = None;
            patch.duration_seconds = self.duration_for(draft)?;
            sqlx::query("UPDATE recurrence_exceptions SET exception_type='override',overrides_json=?,reminders_overridden=1,updated_at=? WHERE id=?")
                .bind(serde_json::to_string(&patch).map_err(error)?).bind(now()).bind(&exception_id).execute(&mut *tx).await.map_err(error)?;
            sync_reminders(&mut tx, identifier, Some(&exception_id), &draft.reminders).await?;
        } else if recurring && scope == Scope::Future && before > 0 {
            sqlx::query("UPDATE calendar_entries SET series_end_before=?,updated_at=? WHERE id=?")
                .bind(original)
                .bind(now())
                .bind(identifier)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
            let mut next = new_entry(draft);
            next.duration_seconds = self.duration_for(draft)?;
            next.series_parent_id = Some(identifier.to_owned());
            if let (Some(old), Some(new)) = (&root.recurrence_rule, &next.recurrence_rule) {
                let old_rule = recurrence::Rule::parse(old)?;
                let mut rule = recurrence::Rule::parse(new)?;
                if old_rule.encode() == rule.encode() {
                    if let Some(total) = old_rule.count {
                        rule.count = Some(total.saturating_sub(before).max(1));
                        next.recurrence_rule = Some(rule.encode());
                    }
                }
            }
            write_entry(&mut tx, &next).await?;
            sync_reminders(&mut tx, &next.id, None, &draft.reminders).await?;
            let exceptions=sqlx::query_as::<_,Exception>("SELECT * FROM recurrence_exceptions WHERE entry_id=? AND original_start>=? ORDER BY original_start DESC").bind(identifier).bind(original).fetch_all(&mut *tx).await.map_err(error)?;
            if next.recurrence_rule.is_none() {
                sqlx::query(
                    "DELETE FROM recurrence_exceptions WHERE entry_id=? AND original_start>=?",
                )
                .bind(identifier)
                .bind(original)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
            } else {
                sqlx::query("PRAGMA defer_foreign_keys=ON")
                    .execute(&mut *tx)
                    .await
                    .map_err(error)?;
                let delta = local_start(&next)? - at;
                for ex in exceptions {
                    let key = rekey(&ex.original_start, delta, next.start_time.is_some())?;
                    let patch =
                        shift_patch(ex.overrides_json.as_deref(), delta, self.zone.as_ref())?;
                    sqlx::query("UPDATE recurrence_exceptions SET entry_id=?,original_start=?,overrides_json=? WHERE id=?").bind(&next.id).bind(key).bind(patch).bind(&ex.id).execute(&mut *tx).await.map_err(error)?;
                    sqlx::query("UPDATE reminders SET entry_id=? WHERE exception_id=?")
                        .bind(&next.id)
                        .bind(&ex.id)
                        .execute(&mut *tx)
                        .await
                        .map_err(error)?;
                    if ex.original_start == original {
                        sqlx::query("DELETE FROM reminders WHERE exception_id=?")
                            .bind(&ex.id)
                            .execute(&mut *tx)
                            .await
                            .map_err(error)?;
                        sqlx::query("UPDATE recurrence_exceptions SET exception_type='completion',overrides_json=NULL,reminders_overridden=0,completed=COALESCE(completed,0) WHERE id=?").bind(&ex.id).execute(&mut *tx).await.map_err(error)?;
                    }
                }
            }
        } else {
            let mut changed = root.clone();
            draft.apply(&mut changed);
            changed.updated_at = now();
            let delta = local_start(&changed)? - local_start(&details.occurrence.entry)?;
            let current = &details.occurrence.entry;
            let schedule_unchanged = draft.start_date == current.start_date
                && draft.start_time == current.start_time
                && draft.end_date == current.end_date
                && draft.end_time == current.end_time
                && draft.all_day == (current.all_day == 1)
                && draft.timezone == current.timezone;
            let duration = if schedule_unchanged {
                match root.duration_seconds {
                    Some(value) => Some(value),
                    None => self.duration_for(&draft_from(&root, vec![]))?,
                }
            } else {
                self.duration_for(draft)?
            };
            let requested_end = local_end(&changed)?;
            let current_end = local_end(&details.occurrence.entry)?;
            let root_end = local_end(&root)?;
            changed = recurrence::shift_entry(&changed, local_start(&root)? + delta)?;
            if let (Some(requested), Some(current), Some(root_end)) =
                (requested_end, current_end, root_end)
            {
                let end = root_end + (requested - current);
                changed.end_date = Some(end.date().to_string());
                changed.end_time = draft
                    .end_time
                    .as_ref()
                    .map(|_| end.format("%H:%M").to_string());
            }
            changed.duration_seconds = duration;
            if let Some(duration) = duration {
                let start=self.zone.resolve(local_start(&changed)?,&changed.timezone)?.ok_or("O novo horário não existe na data inicial da série. Use outra data ou edite esta e as próximas ocorrências.")?;
                let end = self
                    .zone
                    .local(start + Duration::seconds(duration), &changed.timezone)?;
                changed.end_date = Some(end.date().to_string());
                changed.end_time = Some(end.format("%H:%M").to_string());
            }
            if changed.recurrence_rule.is_none() {
                changed.series_end_before = None;
            }
            write_entry(&mut tx, &changed).await?;
            sync_reminders(&mut tx, identifier, None, &draft.reminders).await?;
            if changed.recurrence_rule.is_none() {
                sqlx::query("DELETE FROM recurrence_exceptions WHERE entry_id=?")
                    .bind(identifier)
                    .execute(&mut *tx)
                    .await
                    .map_err(error)?;
            } else if delta != Duration::zero() {
                let exceptions = sqlx::query_as::<_, Exception>(
                    "SELECT * FROM recurrence_exceptions WHERE entry_id=?",
                )
                .bind(identifier)
                .fetch_all(&mut *tx)
                .await
                .map_err(error)?;
                // Evita colisões temporárias ao deslocar duas exceções em dias consecutivos.
                for ex in &exceptions {
                    sqlx::query("UPDATE recurrence_exceptions SET original_start=? WHERE id=?")
                        .bind(format!("temporary:{}", ex.id))
                        .bind(&ex.id)
                        .execute(&mut *tx)
                        .await
                        .map_err(error)?;
                }
                for ex in exceptions {
                    sqlx::query("UPDATE recurrence_exceptions SET original_start=?,overrides_json=? WHERE id=?").bind(rekey(&ex.original_start,delta,changed.start_time.is_some())?).bind(shift_patch(ex.overrides_json.as_deref(),delta,self.zone.as_ref())?).bind(&ex.id).execute(&mut *tx).await.map_err(error)?;
                }
                if let Some(cutoff) = &root.series_end_before {
                    sqlx::query("UPDATE calendar_entries SET series_end_before=? WHERE id=?")
                        .bind(rekey(cutoff, delta, changed.start_time.is_some())?)
                        .bind(identifier)
                        .execute(&mut *tx)
                        .await
                        .map_err(error)?;
                }
            }
        }
        cancel_pending(&mut tx, identifier).await?;
        tx.commit().await.map_err(error)
    }
    pub async fn remove(&self, identifier: &str, original: &str, scope: Scope) -> Result<()> {
        let root = self.entry(identifier).await?;
        self.verify_original(&root, original)?;
        let at = key_datetime(original)?;
        let no_previous = scope == Scope::Future
            && recurrence::starts(
                &root,
                date(&root.start_date)?,
                at.date() + Duration::days(1),
                self.zone.as_ref(),
            )?
            .iter()
            .all(|value| *value >= at);
        let mut tx = self.pool.begin().await.map_err(error)?;
        if root.recurrence_rule.is_none() || scope == Scope::Series || no_previous {
            sqlx::query("DELETE FROM calendar_entries WHERE id=?")
                .bind(identifier)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        } else if scope == Scope::Future {
            sqlx::query("UPDATE calendar_entries SET series_end_before=?,updated_at=? WHERE id=?")
                .bind(original)
                .bind(now())
                .bind(identifier)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
            sqlx::query("DELETE FROM recurrence_exceptions WHERE entry_id=? AND original_start>=?")
                .bind(identifier)
                .bind(original)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        } else {
            let ex = ensure_exception(&mut tx, identifier, original).await?;
            sqlx::query("UPDATE recurrence_exceptions SET exception_type='cancelled',updated_at=? WHERE id=?").bind(now()).bind(ex).execute(&mut *tx).await.map_err(error)?;
        }
        cancel_pending(&mut tx, identifier).await?;
        tx.commit().await.map_err(error)
    }
    pub async fn complete(&self, identifier: &str, original: &str, completed: bool) -> Result<()> {
        let root = self.entry(identifier).await?;
        self.verify_original(&root, original)?;
        if self
            .details(identifier, original)
            .await?
            .occurrence
            .entry
            .kind
            != "task"
        {
            return Err("Somente tarefas podem ser concluídas.".into());
        }
        let mut tx = self.pool.begin().await.map_err(error)?;
        if root.recurrence_rule.is_some() {
            let ex = ensure_exception(&mut tx, identifier, original).await?;
            sqlx::query("UPDATE recurrence_exceptions SET completed=?,updated_at=? WHERE id=? AND exception_type<>'cancelled'").bind(i64::from(completed)).bind(now()).bind(ex).execute(&mut *tx).await.map_err(error)?;
        } else {
            sqlx::query("UPDATE calendar_entries SET completed=?,updated_at=? WHERE id=?")
                .bind(i64::from(completed))
                .bind(now())
                .bind(identifier)
                .execute(&mut *tx)
                .await
                .map_err(error)?;
        }
        cancel_pending(&mut tx, identifier).await?;
        tx.commit().await.map_err(error)
    }
    pub async fn duplicate(&self, identifier: &str, original: &str) -> Result<Entry> {
        let details = self.details(identifier, original).await?;
        let mut draft = draft_from(&details.occurrence.entry, details.reminders);
        draft.title = format!(
            "{} (cópia)",
            draft.title.chars().take(292).collect::<String>()
        );
        draft.recurrence_rule = None;
        for reminder in &mut draft.reminders {
            reminder.id = None;
        }
        self.create(&draft).await
    }
}

async fn ensure_exception(
    tx: &mut Transaction<'_, Sqlite>,
    entry: &str,
    original: &str,
) -> Result<String> {
    let identifier = id();
    sqlx::query("INSERT INTO recurrence_exceptions(id,entry_id,original_start,exception_type,completed) VALUES(?,?,?,'completion',0) ON CONFLICT(entry_id,original_start) DO NOTHING").bind(identifier).bind(entry).bind(original).execute(&mut **tx).await.map_err(error)?;
    sqlx::query_scalar("SELECT id FROM recurrence_exceptions WHERE entry_id=? AND original_start=?")
        .bind(entry)
        .bind(original)
        .fetch_one(&mut **tx)
        .await
        .map_err(error)
}
async fn cancel_pending(tx: &mut Transaction<'_, Sqlite>, entry: &str) -> Result<()> {
    sqlx::query("UPDATE notification_deliveries SET state='cancelled' WHERE reminder_id IN (SELECT id FROM reminders WHERE entry_id=?) AND state IN ('pending','failed')").bind(entry).execute(&mut **tx).await.map_err(error)?;
    Ok(())
}
async fn sync_reminders(
    tx: &mut Transaction<'_, Sqlite>,
    entry: &str,
    exception: Option<&str>,
    drafts: &[ReminderDraft],
) -> Result<()> {
    let existing = sqlx::query_as::<_, Reminder>(
        "SELECT * FROM reminders WHERE entry_id=? AND exception_id IS ?",
    )
    .bind(entry)
    .bind(exception)
    .fetch_all(&mut **tx)
    .await
    .map_err(error)?;
    let mut kept = HashSet::new();
    for draft in drafts {
        let identifier = draft
            .id
            .as_ref()
            .filter(|identifier| existing.iter().any(|r| &r.id == *identifier))
            .cloned()
            .unwrap_or_else(id);
        if !kept.insert(identifier.clone()) {
            return Err("Lembrete duplicado.".into());
        }
        sqlx::query("INSERT INTO reminders(id,entry_id,exception_id,kind,minutes_before,scheduled_at) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind,minutes_before=excluded.minutes_before,scheduled_at=excluded.scheduled_at,enabled=1,updated_at=?").bind(&identifier).bind(entry).bind(exception).bind(&draft.kind).bind(draft.minutes_before).bind(&draft.scheduled_at).bind(now()).execute(&mut **tx).await.map_err(error)?;
    }
    for reminder in existing {
        if !kept.contains(&reminder.id) {
            sqlx::query("DELETE FROM reminders WHERE id=?")
                .bind(reminder.id)
                .execute(&mut **tx)
                .await
                .map_err(error)?;
        }
    }
    Ok(())
}
fn rekey(key: &str, delta: Duration, timed: bool) -> Result<String> {
    let at = key_datetime(key)? + delta;
    Ok(if timed {
        at.format("%Y-%m-%dT%H:%M").to_string()
    } else {
        at.date().to_string()
    })
}
fn shift_patch(
    patch: Option<&str>,
    delta: Duration,
    zone: &dyn TimeResolver,
) -> Result<Option<String>> {
    let Some(patch) = patch else { return Ok(None) };
    let mut draft: EntryDraft = serde_json::from_str(patch).map_err(error)?;
    let original = new_entry(&draft);
    let changed = recurrence::shift_entry(&original, local_start(&original)? + delta)?;
    draft.start_date = changed.start_date;
    draft.start_time = changed.start_time;
    draft.end_date = changed.end_date;
    draft.end_time = changed.end_time;
    draft.start_utc = None;
    draft.end_utc = None;
    if let Some(duration) = draft.duration_seconds {
        let start = zone
            .resolve(local_start(&new_entry(&draft))?, &draft.timezone)?
            .ok_or("Uma exceção seria movida para um horário inexistente neste fuso.")?;
        let end = start + Duration::seconds(duration);
        let local_end = zone.local(end, &draft.timezone)?;
        draft.end_date = Some(local_end.date().to_string());
        draft.end_time = Some(local_end.format("%H:%M").to_string());
        draft.start_utc = Some(start.to_rfc3339());
        draft.end_utc = Some(end.to_rfc3339());
    }
    Ok(Some(serde_json::to_string(&draft).map_err(error)?))
}
