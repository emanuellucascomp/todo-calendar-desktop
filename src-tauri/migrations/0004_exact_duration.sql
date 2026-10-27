-- Preserva duração exata de eventos durante repetições do relógio (RFC 5545).

-- Executada dentro da transação da migration; todas as relações são copiadas e restauradas.

PRAGMA defer_foreign_keys=ON;

CREATE TEMP TABLE _tc_copy_calendar_entries AS SELECT *, NULL AS duration_seconds FROM calendar_entries;

CREATE TEMP TABLE _tc_copy_recurrence_exceptions AS SELECT * FROM recurrence_exceptions;

CREATE TEMP TABLE _tc_copy_reminders AS SELECT * FROM reminders;

CREATE TEMP TABLE _tc_copy_notification_deliveries AS SELECT * FROM notification_deliveries;

DROP TABLE notification_deliveries;

DROP TABLE reminders;

DROP TABLE recurrence_exceptions;

DROP TABLE calendar_entries;

CREATE TABLE calendar_entries (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('task', 'event')),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 300),
    description TEXT NOT NULL DEFAULT '',
    start_date TEXT NOT NULL CHECK (length(start_date) = 10 AND date(start_date, '+0 days') IS NOT NULL AND date(start_date, '+0 days') = start_date),
    start_time TEXT CHECK (start_time IS NULL OR (length(start_time) = 5 AND start_time GLOB '[0-2][0-9]:[0-5][0-9]' AND start_time < '24:00')),
    end_date TEXT CHECK (end_date IS NULL OR (length(end_date) = 10 AND date(end_date, '+0 days') IS NOT NULL AND date(end_date, '+0 days') = end_date)),
    end_time TEXT CHECK (end_time IS NULL OR (length(end_time) = 5 AND end_time GLOB '[0-2][0-9]:[0-5][0-9]' AND end_time < '24:00')),
    all_day INTEGER NOT NULL DEFAULT 0 CHECK (all_day IN (0, 1)),
    timezone TEXT NOT NULL,
    category_id TEXT REFERENCES categories(id) ON DELETE SET NULL,
    priority TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high')),
    completed INTEGER NOT NULL DEFAULT 0 CHECK (completed IN (0, 1)),
    recurrence_rule TEXT,
    series_parent_id TEXT REFERENCES calendar_entries(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    series_end_before TEXT,
    duration_seconds INTEGER CHECK (duration_seconds IS NULL OR (duration_seconds BETWEEN 1 AND 315360000000 AND all_day=0 AND start_time IS NOT NULL AND end_time IS NOT NULL AND end_date IS NOT NULL)),
    CHECK (kind = 'task' OR completed = 0),
    CHECK (all_day = 0 OR (start_time IS NULL AND end_time IS NULL)),
    CHECK (end_time IS NULL OR (end_date IS NOT NULL AND start_time IS NOT NULL)),
    CHECK (duration_seconds IS NOT NULL OR end_date IS NULL OR end_date >= start_date),
    CHECK (duration_seconds IS NOT NULL OR end_date IS NULL OR end_time IS NULL OR end_date > start_date OR end_time > start_time),
    CHECK (kind = 'task' OR (end_date IS NOT NULL AND ((all_day = 1 AND end_date > start_date) OR (all_day = 0 AND start_time IS NOT NULL AND end_time IS NOT NULL)))),
    CHECK (series_parent_id IS NULL OR series_parent_id <> id)
);

CREATE TABLE recurrence_exceptions (
    id TEXT PRIMARY KEY NOT NULL,
    entry_id TEXT NOT NULL REFERENCES calendar_entries(id) ON DELETE CASCADE,
    original_start TEXT NOT NULL,
    exception_type TEXT NOT NULL CHECK (exception_type IN ('override', 'cancelled', 'completion')),
    overrides_json TEXT CHECK (overrides_json IS NULL OR (json_valid(overrides_json) AND json_type(overrides_json) = 'object')),
    completed INTEGER CHECK (completed IS NULL OR completed IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    reminders_overridden INTEGER NOT NULL DEFAULT 0 CHECK (reminders_overridden IN (0,1)),
    UNIQUE (entry_id, original_start),
    UNIQUE (id, entry_id),
    CHECK (exception_type <> 'completion' OR completed IS NOT NULL),
    CHECK (exception_type <> 'override' OR overrides_json IS NOT NULL)
);

CREATE TABLE reminders (
    id TEXT PRIMARY KEY NOT NULL,
    entry_id TEXT NOT NULL REFERENCES calendar_entries(id) ON DELETE CASCADE,
    exception_id TEXT,
    kind TEXT NOT NULL CHECK (kind IN ('relative', 'absolute')),
    minutes_before INTEGER CHECK (minutes_before IS NULL OR minutes_before >= 0),
    scheduled_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((kind = 'relative' AND minutes_before IS NOT NULL AND scheduled_at IS NULL) OR (kind = 'absolute' AND scheduled_at IS NOT NULL AND minutes_before IS NULL)),
    FOREIGN KEY (exception_id, entry_id) REFERENCES recurrence_exceptions(id, entry_id) ON DELETE CASCADE
);

CREATE TABLE notification_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    reminder_id TEXT NOT NULL REFERENCES reminders(id) ON DELETE CASCADE,
    occurrence_key TEXT NOT NULL,
    scheduled_at TEXT NOT NULL,
    delivered_at TEXT,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'claimed', 'delivered', 'cancelled', 'failed')),
    claimed_at TEXT,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts>=0),
    UNIQUE (reminder_id, occurrence_key)
);

INSERT INTO calendar_entries(id,kind,title,description,start_date,start_time,end_date,end_time,all_day,timezone,category_id,priority,completed,recurrence_rule,series_parent_id,created_at,updated_at,series_end_before,duration_seconds) SELECT id,kind,title,description,start_date,start_time,end_date,end_time,all_day,timezone,category_id,priority,completed,recurrence_rule,series_parent_id,created_at,updated_at,series_end_before,duration_seconds FROM _tc_copy_calendar_entries;

INSERT INTO recurrence_exceptions(id,entry_id,original_start,exception_type,overrides_json,completed,created_at,updated_at,reminders_overridden) SELECT id,entry_id,original_start,exception_type,overrides_json,completed,created_at,updated_at,reminders_overridden FROM _tc_copy_recurrence_exceptions;

INSERT INTO reminders(id,entry_id,exception_id,kind,minutes_before,scheduled_at,enabled,created_at,updated_at) SELECT id,entry_id,exception_id,kind,minutes_before,scheduled_at,enabled,created_at,updated_at FROM _tc_copy_reminders;

INSERT INTO notification_deliveries(id,reminder_id,occurrence_key,scheduled_at,delivered_at,state,claimed_at,error,created_at,attempts) SELECT id,reminder_id,occurrence_key,scheduled_at,delivered_at,state,claimed_at,error,created_at,attempts FROM _tc_copy_notification_deliveries;

CREATE INDEX idx_entries_start ON calendar_entries(start_date, end_date);

CREATE INDEX idx_entries_category ON calendar_entries(category_id);

CREATE INDEX idx_entries_parent ON calendar_entries(series_parent_id);

CREATE INDEX idx_entries_recurrence ON calendar_entries(start_date) WHERE recurrence_rule IS NOT NULL;

CREATE INDEX idx_exceptions_original ON recurrence_exceptions(original_start);

CREATE INDEX idx_reminders_entry ON reminders(entry_id);

CREATE INDEX idx_reminders_exception ON reminders(exception_id);

CREATE INDEX idx_deliveries_due ON notification_deliveries(state, scheduled_at);

CREATE INDEX idx_entries_cutoff ON calendar_entries(series_end_before) WHERE series_end_before IS NOT NULL;

DROP TABLE _tc_copy_notification_deliveries;

DROP TABLE _tc_copy_reminders;

DROP TABLE _tc_copy_recurrence_exceptions;

DROP TABLE _tc_copy_calendar_entries;
