ALTER TABLE calendar_entries ADD COLUMN series_end_before TEXT;
ALTER TABLE recurrence_exceptions ADD COLUMN reminders_overridden INTEGER NOT NULL DEFAULT 0 CHECK (reminders_overridden IN (0, 1));
CREATE INDEX idx_entries_cutoff ON calendar_entries(series_end_before) WHERE series_end_before IS NOT NULL;
INSERT OR IGNORE INTO app_settings(key,value) VALUES ('scheduler_last_check', 'null');
