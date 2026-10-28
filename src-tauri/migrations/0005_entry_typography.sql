-- A fonte passa a pertencer ao registro; os existentes conservam a aparência anterior.
ALTER TABLE calendar_entries ADD COLUMN font_size INTEGER NOT NULL DEFAULT 12 CHECK(font_size BETWEEN 10 AND 24);
ALTER TABLE calendar_entries ADD COLUMN bold INTEGER NOT NULL DEFAULT 0 CHECK(bold IN (0,1));
ALTER TABLE calendar_entries ADD COLUMN italic INTEGER NOT NULL DEFAULT 0 CHECK(italic IN (0,1));
ALTER TABLE calendar_entries ADD COLUMN underline INTEGER NOT NULL DEFAULT 0 CHECK(underline IN (0,1));
UPDATE calendar_entries SET
    font_size = COALESCE((SELECT CAST(value AS INTEGER) FROM app_settings WHERE key='calendar_font_size'),12),
    bold = COALESCE((SELECT value='true' FROM app_settings WHERE key='calendar_bold'),0),
    italic = COALESCE((SELECT value='true' FROM app_settings WHERE key='calendar_italic'),0),
    underline = COALESCE((SELECT value='true' FROM app_settings WHERE key='calendar_underline'),0);
UPDATE recurrence_exceptions SET overrides_json = json_set(overrides_json,
    '$.font_size', (SELECT font_size FROM calendar_entries WHERE id=entry_id),
    '$.bold', json(CASE WHEN (SELECT bold FROM calendar_entries WHERE id=entry_id) THEN 'true' ELSE 'false' END),
    '$.italic', json(CASE WHEN (SELECT italic FROM calendar_entries WHERE id=entry_id) THEN 'true' ELSE 'false' END),
    '$.underline', json(CASE WHEN (SELECT underline FROM calendar_entries WHERE id=entry_id) THEN 'true' ELSE 'false' END))
WHERE overrides_json IS NOT NULL;
