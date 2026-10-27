use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::{future::Future, sync::Arc};
use todo_calendar_core::{
    models::*,
    recurrence::{self, TimeResolver},
    repository::{draft_from, Repository},
    Result,
};

struct UTC;

#[test]
fn timed_until_is_stored_in_utc_and_remains_inclusive() {
    run(async {
        let repo = repo().await;
        let mut input = draft(Some("FREQ=DAILY;UNTIL=20261104"));
        input.start_time = Some("09:00".into());
        let root = repo.create(&input).await.unwrap();
        assert!(root
            .recurrence_rule
            .as_deref()
            .unwrap()
            .contains("UNTIL=20261104T235959Z"));
        assert_eq!(
            repo.occurrences("2026-11-01", "2026-11-10")
                .await
                .unwrap()
                .len(),
            3
        );
    })
}

#[test]
fn recurring_event_occurrence_can_become_an_independent_task() {
    run(async {
        let repo = repo().await;
        let mut input = draft(Some("FREQ=DAILY;COUNT=2"));
        input.kind = "event".into();
        input.start_time = Some("09:00".into());
        input.end_date = Some("2026-11-02".into());
        input.end_time = Some("10:00".into());
        let root = repo.create(&input).await.unwrap();
        repo.update(&root.id, "2026-11-02T09:00", Scope::One, &draft(None))
            .await
            .unwrap();
        repo.complete(&root.id, "2026-11-02T09:00", true)
            .await
            .unwrap();
        let rows = repo.occurrences("2026-11-02", "2026-11-04").await.unwrap();
        assert_eq!(rows[0].entry.kind, "task");
        assert_eq!(rows[0].entry.completed, 1);
        assert_eq!(rows[1].entry.kind, "event");
        assert_eq!(rows[1].entry.completed, 0);
    })
}

#[test]
fn fall_back_occurrence_keeps_utc_duration_in_single_and_future_edits() {
    run(async {
        use chrono::Timelike;
        struct Fall;
        impl TimeResolver for Fall {
            fn resolve(&self, at: NaiveDateTime, _: &str) -> Result<Option<DateTime<Utc>>> {
                let boundary = date("2026-11-01").unwrap();
                let offset = if at.date() < boundary || (at.date() == boundary && at.hour() < 2) {
                    4
                } else {
                    5
                };
                Ok(Some((at + chrono::Duration::hours(offset)).and_utc()))
            }
            fn local(&self, at: DateTime<Utc>, _: &str) -> Result<NaiveDateTime> {
                let boundary = DateTime::parse_from_rfc3339("2026-11-01T06:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc);
                Ok((at - chrono::Duration::hours(if at < boundary { 4 } else { 5 })).naive_utc())
            }
        }
        let mut repo = repo().await;
        repo.zone = Arc::new(Fall);
        let mut input = draft(Some("FREQ=DAILY;COUNT=2"));
        input.kind = "event".into();
        input.start_date = "2026-10-31".into();
        input.start_time = Some("01:45".into());
        input.end_date = Some("2026-10-31".into());
        input.end_time = Some("02:15".into());
        let root = repo.create(&input).await.unwrap();
        let details = repo.details(&root.id, "2026-11-01T01:45").await.unwrap();
        assert_eq!(details.occurrence.entry.end_time.as_deref(), Some("01:15"));
        assert_eq!(
            details.occurrence.end_utc.as_deref(),
            Some("2026-11-01T06:15:00+00:00")
        );
        let mut one = draft_from(&details.occurrence.entry, vec![]);
        one.recurrence_rule = None;
        one.title = "Esta ocorrência".into();
        repo.update(&root.id, "2026-11-01T01:45", Scope::One, &one)
            .await
            .unwrap();
        let actual = repo.details(&root.id, "2026-11-01T01:45").await.unwrap();
        assert_eq!(actual.occurrence.end_utc, details.occurrence.end_utc);
        one.recurrence_rule = input.recurrence_rule.clone();
        one.title = "Novo futuro".into();
        repo.update(&root.id, "2026-11-01T01:45", Scope::Future, &one)
            .await
            .unwrap();
        let list = repo.occurrences("2026-10-31", "2026-11-02").await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].entry.duration_seconds, Some(1800));
        assert_eq!(list[1].entry.title, "Novo futuro");
        assert!(repo
            .occurrences("2026-11-02", "2026-11-03")
            .await
            .unwrap()
            .is_empty());
        repo.export_backup().await.unwrap().validate().unwrap();
    })
}

#[test]
fn recurring_dtend_preserves_exact_duration_across_dst() {
    run(async {
        use chrono::Timelike;
        struct Transition;
        impl TimeResolver for Transition {
            fn resolve(&self, at: NaiveDateTime, _: &str) -> Result<Option<DateTime<Utc>>> {
                let boundary = date("2026-03-08").unwrap();
                let offset = if at.date() < boundary || (at.date() == boundary && at.hour() < 2) {
                    5
                } else {
                    4
                };
                if at.date() == boundary && at.hour() == 2 {
                    return Ok(None);
                }
                Ok(Some((at + chrono::Duration::hours(offset)).and_utc()))
            }
            fn local(&self, at: DateTime<Utc>, _: &str) -> Result<NaiveDateTime> {
                let boundary = DateTime::parse_from_rfc3339("2026-03-08T07:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc);
                Ok((at - chrono::Duration::hours(if at < boundary { 5 } else { 4 })).naive_utc())
            }
        }
        let mut repo = repo().await;
        repo.zone = Arc::new(Transition);
        let mut input = draft(Some("FREQ=DAILY;COUNT=2"));
        input.kind = "event".into();
        input.start_date = "2026-03-07".into();
        input.start_time = Some("01:30".into());
        input.end_date = Some("2026-03-07".into());
        input.end_time = Some("03:30".into());
        repo.create(&input).await.unwrap();
        let occurrences = repo.occurrences("2026-03-07", "2026-03-09").await.unwrap();
        assert_eq!(occurrences[0].entry.end_time.as_deref(), Some("03:30"));
        assert_eq!(occurrences[1].entry.end_time.as_deref(), Some("04:30"));
    })
}
#[test]
fn editing_series_title_from_an_override_does_not_change_base_duration() {
    run(async {
        let repo = repo().await;
        let mut input = draft(Some("FREQ=WEEKLY;COUNT=2"));
        input.kind = "event".into();
        input.start_time = Some("09:00".into());
        input.end_date = Some("2026-11-02".into());
        input.end_time = Some("10:00".into());
        let root = repo.create(&input).await.unwrap();
        let mut one = input.clone();
        one.recurrence_rule = None;
        one.start_date = "2026-11-09".into();
        one.end_date = Some("2026-11-09".into());
        one.end_time = Some("12:00".into());
        repo.update(&root.id, "2026-11-09T09:00", Scope::One, &one)
            .await
            .unwrap();
        let details = repo.details(&root.id, "2026-11-09T09:00").await.unwrap();
        let mut all = draft_from(&details.occurrence.entry, details.reminders);
        all.title = "Novo título".into();
        repo.update(&root.id, "2026-11-09T09:00", Scope::Series, &all)
            .await
            .unwrap();
        assert_eq!(
            repo.entry(&root.id).await.unwrap().end_time.as_deref(),
            Some("10:00")
        );
    })
}

#[test]
fn utc_until_does_not_end_series_at_dst_gap() {
    run(async {
        struct Gap;
        impl TimeResolver for Gap {
            fn resolve(&self, at: NaiveDateTime, _: &str) -> Result<Option<DateTime<Utc>>> {
                Ok(if at.date().to_string() == "2026-11-03" {
                    None
                } else {
                    Some(at.and_utc())
                })
            }
        }
        let repo = repo().await;
        let mut input = draft(Some("FREQ=DAILY;UNTIL=20261104T235959Z"));
        input.start_time = Some("02:30".into());
        let root = repo.create(&input).await.unwrap();
        let starts = recurrence::starts(
            &root,
            date("2026-11-01").unwrap(),
            date("2026-11-10").unwrap(),
            &Gap,
        )
        .unwrap();
        assert_eq!(
            starts
                .iter()
                .map(|v| v.date().to_string())
                .collect::<Vec<_>>(),
            ["2026-11-02", "2026-11-04"]
        );
    })
}
#[test]
fn reminder_insert_failure_rolls_back_the_entry() {
    run(async {
        let repo = repo().await;
        sqlx::raw_sql("CREATE TRIGGER fail_reminder BEFORE INSERT ON reminders BEGIN SELECT RAISE(ABORT,'Falha de escrita simulada'); END;").execute(&repo.pool).await.unwrap();
        let mut input = draft(None);
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(10),
            scheduled_at: None,
        }];
        assert!(repo.create(&input).await.is_err());
        assert!(repo.entries().await.unwrap().is_empty());
        assert!(repo.reminders().await.unwrap().is_empty());
    })
}
#[test]
fn deleting_category_cleans_individual_overrides() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=DAILY;COUNT=2")))
            .await
            .unwrap();
        let mut one = draft(None);
        one.title = "Exceção".into();
        repo.update(&root.id, "2026-11-02", Scope::One, &one)
            .await
            .unwrap();
        repo.delete_category("personal").await.unwrap();
        assert_eq!(repo.entry(&root.id).await.unwrap().category_id, None);
        assert_eq!(
            repo.details(&root.id, "2026-11-02")
                .await
                .unwrap()
                .occurrence
                .entry
                .category_id,
            None
        );
        repo.export_backup().await.unwrap().validate().unwrap();
    })
}
#[test]
fn absolute_occurrence_reminder_can_precede_the_series_by_years() {
    run(async {
        let repo = repo().await;
        let mut input = draft(Some("FREQ=YEARLY;COUNT=3"));
        input.start_date = "2099-01-01".into();
        let root = repo.create(&input).await.unwrap();
        input.recurrence_rule = None;
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "absolute".into(),
            minutes_before: None,
            scheduled_at: Some("2026-11-02T10:00:00Z".into()),
        }];
        repo.update(&root.id, "2099-01-01", Scope::One, &input)
            .await
            .unwrap();
        sqlx::query("UPDATE calendar_entries SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE reminders SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE recurrence_exceptions SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        let current = DateTime::parse_from_rfc3339("2026-11-02T10:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            repo.due_deliveries(current, current - chrono::Duration::hours(24))
                .await
                .unwrap()
                .len(),
            1
        );
    })
}
#[test]
fn import_failure_restores_all_previous_data() {
    run(async {
        let repo = repo().await;
        repo.create(&draft(None)).await.unwrap();
        let backup = repo.export_backup().await.unwrap();
        let mut next = draft(None);
        next.title = "Preservar".into();
        repo.create(&next).await.unwrap();
        sqlx::raw_sql("CREATE TRIGGER fail_restore BEFORE INSERT ON calendar_entries BEGIN SELECT RAISE(ABORT,'Falha na restauração simulada'); END;").execute(&repo.pool).await.unwrap();
        assert!(repo.import_backup(&backup).await.is_err());
        assert_eq!(repo.entries().await.unwrap().len(), 2);
        assert!(repo
            .entries()
            .await
            .unwrap()
            .iter()
            .any(|entry| entry.title == "Preservar"));
    })
}
#[test]
fn unrelated_completion_never_replays_a_claimed_notification() {
    run(async {
        let repo = repo().await;
        let mut input = draft(Some("FREQ=DAILY;COUNT=2"));
        input.start_time = Some("10:00".into());
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(0),
            scheduled_at: None,
        }];
        let root = repo.create(&input).await.unwrap();
        sqlx::query("UPDATE calendar_entries SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE reminders SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        let current = DateTime::parse_from_rfc3339("2026-11-02T10:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let due = repo
            .due_deliveries(current, current - chrono::Duration::hours(24))
            .await
            .unwrap();
        assert!(repo.claim_delivery(&due[0]).await.unwrap());
        repo.complete(&root.id, "2026-11-03T10:00", true)
            .await
            .unwrap();
        assert!(!repo.claim_delivery(&due[0]).await.unwrap());
    })
}
impl TimeResolver for UTC {
    fn resolve(&self, local: NaiveDateTime, _: &str) -> Result<Option<DateTime<Utc>>> {
        Ok(Some(local.and_utc()))
    }
}
fn run(test: impl Future<Output = ()>) {
    tokio::runtime::Runtime::new().unwrap().block_on(test)
}
async fn repo() -> Repository {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .in_memory(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    for migration in todo_calendar_core::MIGRATIONS {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    Repository {
        pool,
        zone: Arc::new(UTC),
    }
}
fn draft(rule: Option<&str>) -> EntryDraft {
    EntryDraft {
        kind: "task".into(),
        title: "Estudar".into(),
        description: String::new(),
        start_date: "2026-11-02".into(),
        start_time: None,
        end_date: None,
        end_time: None,
        all_day: false,
        timezone: "UTC".into(),
        category_id: Some("personal".into()),
        priority: "medium".into(),
        recurrence_rule: rule.map(str::to_owned),
        reminders: vec![],
        start_utc: None,
        end_utc: None,
        duration_seconds: None,
    }
}

#[test]
fn create_edit_duplicate_and_delete() {
    run(async {
        let repo = repo().await;
        let root = repo.create(&draft(None)).await.unwrap();
        let mut edit = draft(None);
        edit.title = "Revisar".into();
        repo.update(&root.id, "2026-11-02", Scope::Series, &edit)
            .await
            .unwrap();
        assert_eq!(repo.entry(&root.id).await.unwrap().title, "Revisar");
        let copy = repo.duplicate(&root.id, "2026-11-02").await.unwrap();
        assert_ne!(copy.id, root.id);
        assert!(copy.title.contains("cópia"));
        repo.complete(&root.id, "2026-11-02", true).await.unwrap();
        assert_eq!(repo.entry(&root.id).await.unwrap().completed, 1);
        repo.remove(&root.id, "2026-11-02", Scope::Series)
            .await
            .unwrap();
        assert_eq!(repo.entries().await.unwrap().len(), 1);
    })
}
#[test]
fn validation_and_atomic_rollback() {
    run(async {
        let repo = repo().await;
        let mut invalid = draft(None);
        invalid.title = " ".into();
        assert!(repo.create(&invalid).await.is_err());
        invalid = draft(None);
        invalid.start_date = "2026-02-30".into();
        assert!(repo.create(&invalid).await.is_err());
        let mut invalid = draft(None);
        invalid.category_id = Some("missing".into());
        invalid.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(10),
            scheduled_at: None,
        }];
        assert!(repo.create(&invalid).await.is_err());
        assert!(repo.entries().await.unwrap().is_empty());
        assert!(repo.reminders().await.unwrap().is_empty());
    })
}
#[test]
fn daily_weekday_weekly_monthly_and_yearly() {
    run(async {
        let repo = repo().await;
        for (rule, from, to, expected) in [
            (
                "FREQ=DAILY;COUNT=3",
                "2026-11-01",
                "2026-12-01",
                vec!["2026-11-02", "2026-11-03", "2026-11-04"],
            ),
            (
                "FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;COUNT=6",
                "2026-11-01",
                "2026-12-01",
                vec![
                    "2026-11-02",
                    "2026-11-03",
                    "2026-11-04",
                    "2026-11-05",
                    "2026-11-06",
                    "2026-11-09",
                ],
            ),
            (
                "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=4",
                "2026-11-01",
                "2026-12-01",
                vec!["2026-11-02", "2026-11-04", "2026-11-16", "2026-11-18"],
            ),
        ] {
            let root = repo.create(&draft(Some(rule))).await.unwrap();
            let list = recurrence::expand(
                &root,
                &[],
                date(from).unwrap(),
                date(to).unwrap(),
                repo.zone.as_ref(),
            )
            .unwrap();
            assert_eq!(
                list.iter()
                    .map(|o| o.entry.start_date.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        let mut monthly = draft(Some("FREQ=MONTHLY;COUNT=3"));
        monthly.start_date = "2026-01-31".into();
        let root = repo.create(&monthly).await.unwrap();
        let list = recurrence::starts(
            &root,
            date("2026-01-01").unwrap(),
            date("2026-06-01").unwrap(),
            repo.zone.as_ref(),
        )
        .unwrap();
        assert_eq!(
            list.iter()
                .map(|v| v.date().to_string())
                .collect::<Vec<_>>(),
            ["2026-01-31", "2026-03-31", "2026-05-31"]
        );
        let mut yearly = draft(Some("FREQ=YEARLY;COUNT=2"));
        yearly.start_date = "2024-02-29".into();
        let root = repo.create(&yearly).await.unwrap();
        let list = recurrence::starts(
            &root,
            date("2024-01-01").unwrap(),
            date("2029-01-01").unwrap(),
            repo.zone.as_ref(),
        )
        .unwrap();
        assert_eq!(
            list.iter()
                .map(|v| v.date().to_string())
                .collect::<Vec<_>>(),
            ["2024-02-29", "2028-02-29"]
        );
    })
}
#[test]
fn completion_and_deleted_occurrences_are_independent() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=WEEKLY;COUNT=3")))
            .await
            .unwrap();
        repo.complete(&root.id, "2026-11-02", true).await.unwrap();
        let list = repo.occurrences("2026-11-01", "2026-12-01").await.unwrap();
        assert_eq!(
            list.iter().map(|o| o.entry.completed).collect::<Vec<_>>(),
            [1, 0, 0]
        );
        repo.remove(&root.id, "2026-11-09", Scope::One)
            .await
            .unwrap();
        let list = repo.occurrences("2026-11-01", "2026-12-01").await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(repo.entries().await.unwrap().len(), 1);
        assert_eq!(repo.exceptions().await.unwrap().len(), 2);
    })
}
#[test]
fn moved_override_is_visible_outside_original_range() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=WEEKLY;COUNT=3")))
            .await
            .unwrap();
        let mut edit = draft(None);
        edit.start_date = "2027-01-04".into();
        edit.title = "Movida".into();
        repo.update(&root.id, "2026-11-09", Scope::One, &edit)
            .await
            .unwrap();
        let list = repo.occurrences("2027-01-01", "2027-02-01").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].entry.title, "Movida");
        assert_eq!(list[0].original_start, "2026-11-09");
        assert_eq!(
            repo.occurrences("2026-11-01", "2026-12-01")
                .await
                .unwrap()
                .len(),
            2
        );
    })
}
#[test]
fn future_split_preserves_count_completion_and_selected_changes() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=DAILY;COUNT=5")))
            .await
            .unwrap();
        repo.complete(&root.id, "2026-11-05", true).await.unwrap();
        let mut single = draft(None);
        single.start_date = "2026-11-04".into();
        single.title = "Exceção antiga".into();
        repo.update(&root.id, "2026-11-04", Scope::One, &single)
            .await
            .unwrap();
        let mut edit = draft(Some("FREQ=DAILY;COUNT=5"));
        edit.start_date = "2026-11-04".into();
        edit.title = "Novo futuro".into();
        repo.update(&root.id, "2026-11-04", Scope::Future, &edit)
            .await
            .unwrap();
        let list = repo.occurrences("2026-11-01", "2026-12-01").await.unwrap();
        assert_eq!(list.len(), 5);
        assert_eq!(list[0].entry.title, "Estudar");
        assert_eq!(list[2].entry.title, "Novo futuro");
        assert_eq!(list[3].entry.completed, 1);
        assert_eq!(repo.entries().await.unwrap().len(), 2);
    })
}
#[test]
fn whole_series_shift_keeps_consecutive_exceptions() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=DAILY;COUNT=5")))
            .await
            .unwrap();
        repo.complete(&root.id, "2026-11-02", true).await.unwrap();
        repo.complete(&root.id, "2026-11-03", true).await.unwrap();
        let details = repo.details(&root.id, "2026-11-03").await.unwrap();
        let mut edit = draft_from(&details.occurrence.entry, vec![]);
        edit.start_date = "2026-11-04".into();
        edit.start_time = Some("09:00".into());
        repo.update(&root.id, "2026-11-03", Scope::Series, &edit)
            .await
            .unwrap();
        let list = repo.occurrences("2026-11-01", "2026-12-01").await.unwrap();
        assert_eq!(list[0].entry.start_date, "2026-11-03");
        assert_eq!(list[0].entry.completed, 1);
        assert_eq!(list[1].entry.completed, 1);
        assert_eq!(list[0].original_start, "2026-11-03T09:00");
    })
}
#[test]
fn delete_future_and_first_valid_occurrence() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=DAILY;COUNT=5")))
            .await
            .unwrap();
        repo.remove(&root.id, "2026-11-04", Scope::Future)
            .await
            .unwrap();
        assert_eq!(
            repo.occurrences("2026-11-01", "2026-12-01")
                .await
                .unwrap()
                .len(),
            2
        );
        let other = repo
            .create(&draft(Some("FREQ=WEEKLY;BYDAY=WE;COUNT=3")))
            .await
            .unwrap();
        repo.remove(&other.id, "2026-11-04", Scope::Future)
            .await
            .unwrap();
        assert!(repo.entry(&other.id).await.is_err());
    })
}
#[test]
fn stable_occurrence_ids_and_until_inclusive() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=DAILY;UNTIL=20261104")))
            .await
            .unwrap();
        let a = repo.occurrences("2026-11-01", "2026-12-01").await.unwrap();
        let b = repo.occurrences("2026-11-03", "2026-11-05").await.unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!(a[1].occurrence_id, b[0].occurrence_id);
        assert!(a[0].occurrence_id.starts_with(&root.id));
    })
}

#[test]
fn reminders_schedule_claim_and_prevent_duplicate_delivery() {
    run(async {
        let repo = repo().await;
        let mut input = draft(Some("FREQ=DAILY;COUNT=3"));
        input.start_time = Some("10:00".into());
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(10),
            scheduled_at: None,
        }];
        let root = repo.create(&input).await.unwrap();
        sqlx::query("UPDATE calendar_entries SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE reminders SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        let current = DateTime::parse_from_rfc3339("2026-11-02T09:51:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let list = repo
            .due_deliveries(current, current - chrono::Duration::hours(24))
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(
            list[0].scheduled_at.to_rfc3339(),
            "2026-11-02T09:50:00+00:00"
        );
        assert!(repo.claim_delivery(&list[0]).await.unwrap());
        assert!(!repo.claim_delivery(&list[0]).await.unwrap());
        repo.finish_delivery(&list[0], Ok(())).await.unwrap();
        assert!(!repo.claim_delivery(&list[0]).await.unwrap());
        repo.complete(&root.id, "2026-11-03T10:00", true)
            .await
            .unwrap();
        let tomorrow = current + chrono::Duration::days(1);
        assert!(repo
            .due_deliveries(tomorrow, tomorrow - chrono::Duration::hours(24))
            .await
            .unwrap()
            .is_empty());
    })
}
#[test]
fn reminder_cancellation_and_limited_retries() {
    run(async {
        let repo = repo().await;
        let mut input = draft(None);
        input.start_time = Some("10:00".into());
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(0),
            scheduled_at: None,
        }];
        let root = repo.create(&input).await.unwrap();
        sqlx::query("UPDATE calendar_entries SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE reminders SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        let current = DateTime::parse_from_rfc3339("2026-11-02T10:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let list = repo
            .due_deliveries(current, current - chrono::Duration::hours(24))
            .await
            .unwrap();
        for _ in 0..3 {
            assert!(repo.claim_delivery(&list[0]).await.unwrap());
            repo.finish_delivery(&list[0], Err("Sem permissão".into()))
                .await
                .unwrap();
        }
        assert!(!repo.claim_delivery(&list[0]).await.unwrap());
        repo.remove(&root.id, "2026-11-02T10:00", Scope::Series)
            .await
            .unwrap();
        assert!(repo
            .due_deliveries(current, current - chrono::Duration::hours(24))
            .await
            .unwrap()
            .is_empty());
        let deliveries: i64 = sqlx::query_scalar("SELECT count(*) FROM notification_deliveries")
            .fetch_one(&repo.pool)
            .await
            .unwrap();
        assert_eq!(deliveries, 0);
    })
}
#[test]
fn backup_roundtrip_validation_and_autostart_protection() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=WEEKLY;COUNT=3")))
            .await
            .unwrap();
        repo.complete(&root.id, "2026-11-02", true).await.unwrap();
        let backup = repo.export_backup().await.unwrap();
        let bytes = serde_json::to_vec(&backup).unwrap();
        let parsed = todo_calendar_core::backup::Backup::parse(&bytes).unwrap();
        repo.remove(&root.id, "2026-11-02", Scope::Series)
            .await
            .unwrap();
        assert!(repo.entries().await.unwrap().is_empty());
        sqlx::query("UPDATE app_settings SET value='true' WHERE key='autostart'")
            .execute(&repo.pool)
            .await
            .unwrap();
        repo.import_backup(&parsed).await.unwrap();
        assert_eq!(
            repo.occurrences("2026-11-01", "2026-12-01")
                .await
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            repo.details(&root.id, "2026-11-02")
                .await
                .unwrap()
                .occurrence
                .entry
                .completed,
            1
        );
        let autostart: String =
            sqlx::query_scalar("SELECT value FROM app_settings WHERE key='autostart'")
                .fetch_one(&repo.pool)
                .await
                .unwrap();
        assert_eq!(autostart, "true");
        let mut invalid = parsed.clone();
        invalid.entries[0].category_id = Some("missing".into());
        assert!(repo.import_backup(&invalid).await.is_err());
        assert_eq!(repo.entries().await.unwrap().len(), 1);
        assert!(todo_calendar_core::backup::Backup::parse(b"not json").is_err());
    })
}
#[test]
fn settings_are_validated_and_persisted() {
    run(async {
        let repo = repo().await;
        let mut settings = todo_calendar_core::settings::load(&repo.pool)
            .await
            .unwrap();
        assert!(!settings.autostart);
        settings.theme = "dark".into();
        settings.first_day = 1;
        settings.default_view = "year".into();
        todo_calendar_core::settings::save(&repo.pool, &settings)
            .await
            .unwrap();
        assert_eq!(
            todo_calendar_core::settings::load(&repo.pool)
                .await
                .unwrap(),
            settings
        );
        let last: String =
            sqlx::query_scalar("SELECT value FROM app_settings WHERE key='last_view'")
                .fetch_one(&repo.pool)
                .await
                .unwrap();
        assert_eq!(last, "\"year\"");
        settings.first_day = 7;
        assert!(todo_calendar_core::settings::save(&repo.pool, &settings)
            .await
            .is_err());
    })
}
#[test]
fn dst_gap_is_skipped_without_consuming_count() {
    run(async {
        struct Gap;
        impl TimeResolver for Gap {
            fn resolve(&self, at: NaiveDateTime, _: &str) -> Result<Option<DateTime<Utc>>> {
                Ok(if at.date().to_string() == "2026-11-03" {
                    None
                } else {
                    Some(at.and_utc())
                })
            }
        }
        let repo = repo().await;
        let mut input = draft(Some("FREQ=DAILY;COUNT=3"));
        input.start_time = Some("02:30".into());
        let root = repo.create(&input).await.unwrap();
        let starts = recurrence::starts(
            &root,
            date("2026-11-01").unwrap(),
            date("2026-11-10").unwrap(),
            &Gap,
        )
        .unwrap();
        assert_eq!(
            starts
                .iter()
                .map(|v| v.date().to_string())
                .collect::<Vec<_>>(),
            ["2026-11-02", "2026-11-04", "2026-11-05"]
        );
    })
}
#[test]
fn persistence_after_pool_reopen() {
    run(async {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-data");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(format!("{}.db", uuid::Uuid::new_v4()));
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone())
            .await
            .unwrap();
        for migration in todo_calendar_core::MIGRATIONS {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        let repo = Repository {
            pool,
            zone: Arc::new(UTC),
        };
        let root = repo.create(&draft(None)).await.unwrap();
        repo.pool.close().await;
        let reopened = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        let title: String = sqlx::query_scalar("SELECT title FROM calendar_entries WHERE id=?")
            .bind(root.id)
            .fetch_one(&reopened)
            .await
            .unwrap();
        assert_eq!(title, "Estudar");
        reopened.close().await;
        std::fs::remove_file(path).unwrap();
    })
}

#[test]
fn absolute_reminder_is_independent_from_event_year() {
    run(async {
        let repo = repo().await;
        let mut input = draft(None);
        input.start_date = "2099-01-01".into();
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "absolute".into(),
            minutes_before: None,
            scheduled_at: Some("2026-11-02T10:00:00Z".into()),
        }];
        repo.create(&input).await.unwrap();
        sqlx::query("UPDATE calendar_entries SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE reminders SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        let current = DateTime::parse_from_rfc3339("2026-11-02T10:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            repo.due_deliveries(current, current - chrono::Duration::hours(24))
                .await
                .unwrap()
                .len(),
            1
        );
    })
}
#[test]
fn backup_restore_preserves_newer_delivery_history() {
    run(async {
        let repo = repo().await;
        let mut input = draft(None);
        input.start_time = Some("10:00".into());
        input.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(0),
            scheduled_at: None,
        }];
        repo.create(&input).await.unwrap();
        sqlx::query("UPDATE calendar_entries SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE reminders SET updated_at='2026-10-01T00:00:00Z'")
            .execute(&repo.pool)
            .await
            .unwrap();
        let backup = repo.export_backup().await.unwrap();
        let current = DateTime::parse_from_rfc3339("2026-11-02T10:01:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let list = repo
            .due_deliveries(current, current - chrono::Duration::hours(24))
            .await
            .unwrap();
        assert!(repo.claim_delivery(&list[0]).await.unwrap());
        repo.finish_delivery(&list[0], Ok(())).await.unwrap();
        repo.import_backup(&backup).await.unwrap();
        assert!(!repo.claim_delivery(&list[0]).await.unwrap());
    })
}
#[test]
fn split_moves_exception_reminders_without_breaking_foreign_keys() {
    run(async {
        let repo = repo().await;
        let root = repo
            .create(&draft(Some("FREQ=DAILY;COUNT=5")))
            .await
            .unwrap();
        let mut one = draft(None);
        one.start_date = "2026-11-05".into();
        one.reminders = vec![ReminderDraft {
            id: None,
            kind: "relative".into(),
            minutes_before: Some(15),
            scheduled_at: None,
        }];
        repo.update(&root.id, "2026-11-05", Scope::One, &one)
            .await
            .unwrap();
        let mut future = draft(Some("FREQ=DAILY;COUNT=5"));
        future.start_date = "2026-11-04".into();
        repo.update(&root.id, "2026-11-04", Scope::Future, &future)
            .await
            .unwrap();
        let exception = repo
            .exceptions()
            .await
            .unwrap()
            .into_iter()
            .find(|ex| ex.original_start == "2026-11-05")
            .unwrap();
        assert_ne!(exception.entry_id, root.id);
        let details = repo
            .details(&exception.entry_id, "2026-11-05")
            .await
            .unwrap();
        assert_eq!(details.reminders.len(), 1);
        assert_eq!(details.reminders[0].minutes_before, Some(15));
        assert!(sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&repo.pool)
            .await
            .unwrap()
            .is_empty());
    })
}
