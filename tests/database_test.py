"""Testa a migration real com SQLite, sem dependências externas.

Não substitui o teste de migrations/conexões no runtime Tauri.
"""
import json
import re
import sqlite3
import unittest
from pathlib import Path
from uuid import uuid4

ROOT = Path(__file__).resolve().parents[1]
MIGRATION = '\n'.join(path.read_text(encoding='utf-8') for path in sorted((ROOT / 'src-tauri/migrations').glob('*.sql')))


class DatabaseTests(unittest.TestCase):
    def test_duration_migration_preserves_existing_relations_and_history(self):
        legacy=sqlite3.connect(':memory:')
        try:
            legacy.execute('PRAGMA foreign_keys=ON')
            files=sorted((ROOT/'src-tauri/migrations').glob('*.sql'))
            legacy.executescript('BEGIN;\n'+'\n'.join(path.read_text(encoding='utf-8') for path in files[:3])+'\nCOMMIT;')
            legacy.execute("INSERT INTO calendar_entries(id,kind,title,start_date,timezone,category_id,recurrence_rule) VALUES('root','task','Série','2026-11-02','UTC','personal','FREQ=DAILY')")
            legacy.execute("INSERT INTO calendar_entries(id,kind,title,start_date,timezone,series_parent_id) VALUES('child','task','Futuro','2026-11-09','UTC','root')")
            legacy.execute("INSERT INTO recurrence_exceptions(id,entry_id,original_start,exception_type,completed) VALUES('ex','root','2026-11-03','completion',1)")
            legacy.execute("INSERT INTO reminders(id,entry_id,exception_id,kind,minutes_before) VALUES('rem','root','ex','relative',10)")
            legacy.execute("INSERT INTO notification_deliveries(id,reminder_id,occurrence_key,scheduled_at,delivered_at,state,attempts) VALUES('del','rem','root@2026-11-03','2026-11-03T08:50:00Z','2026-11-03T08:50:01Z','delivered',1)")
            legacy.commit()
            legacy.executescript('BEGIN;\n'+files[3].read_text(encoding='utf-8')+'\nCOMMIT;')
            self.assertEqual(legacy.execute('SELECT count(*) FROM calendar_entries').fetchone()[0],2)
            self.assertEqual(legacy.execute("SELECT series_parent_id FROM calendar_entries WHERE id='child'").fetchone()[0],'root')
            self.assertEqual(legacy.execute('SELECT completed FROM recurrence_exceptions').fetchone()[0],1)
            self.assertEqual(legacy.execute('SELECT exception_id FROM reminders').fetchone()[0],'ex')
            self.assertEqual(legacy.execute('SELECT state,attempts FROM notification_deliveries').fetchone(),('delivered',1))
            self.assertEqual(legacy.execute('PRAGMA foreign_key_check').fetchall(),[])
        finally:
            legacy.close()

    def setUp(self):
        directory = ROOT / '.test-data'
        directory.mkdir(exist_ok=True)
        self.path = directory / f'{uuid4().hex}.db'
        self.db = sqlite3.connect(self.path)
        self.db.execute('PRAGMA foreign_keys = ON')
        self.db.executescript('BEGIN;\n' + MIGRATION + '\nCOMMIT;')

    def tearDown(self):
        self.db.close()
        self.path.unlink(missing_ok=True)

    def task(self, identifier='task', **overrides):
        values = dict(id=identifier, kind='task', title='Estudar Python', start_date='2026-11-02', timezone='America/Sao_Paulo')
        values.update(overrides)
        # Colunas são constantes do teste; valores são sempre parametrizados.
        columns = ', '.join(values.keys())
        placeholders = ', '.join('?' for _ in values)
        self.db.execute(f'INSERT INTO calendar_entries ({columns}) VALUES ({placeholders})', list(values.values()))

    def test_tables_defaults_and_foreign_keys(self):
        names = {row[0] for row in self.db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
        self.assertTrue({'categories', 'calendar_entries', 'recurrence_exceptions', 'reminders', 'notification_deliveries', 'app_settings'} <= names)
        self.assertEqual(self.db.execute('PRAGMA foreign_keys').fetchone()[0], 1)
        settings = {key: json.loads(value) for key, value in self.db.execute('SELECT key, value FROM app_settings')}
        self.assertFalse(settings['autostart'])
        self.assertEqual(settings['last_view'], 'month')
        self.assertEqual(self.db.execute('SELECT count(*) FROM calendar_entries').fetchone()[0], 0)

    def test_real_file_persists_after_reopen(self):
        self.task()
        self.db.commit()
        self.db.close()
        self.db = sqlite3.connect(self.path)
        self.assertEqual(self.db.execute('SELECT title FROM calendar_entries').fetchone()[0], 'Estudar Python')
        self.assertEqual(self.db.execute('PRAGMA integrity_check').fetchone()[0], 'ok')

    def test_update_and_delete(self):
        self.task()
        self.db.execute('UPDATE calendar_entries SET title = ?, completed = ? WHERE id = ?', ('Revisar projeto', 1, 'task'))
        self.assertEqual(self.db.execute('SELECT title, completed FROM calendar_entries').fetchone(), ('Revisar projeto', 1))
        self.db.execute('DELETE FROM calendar_entries WHERE id = ?', ('task',))
        self.assertEqual(self.db.execute('SELECT count(*) FROM calendar_entries').fetchone()[0], 0)

    def test_invalid_entries_are_rejected(self):
        for overrides in [dict(title=' '), dict(start_date='2026-02-30'), dict(start_date='garbage'), dict(start_time='25:30'), dict(priority='urgent'), dict(category_id='missing'), dict(end_date='2026-11-01')]:
            with self.subTest(overrides=overrides):
                with self.assertRaises(sqlite3.IntegrityError):
                    self.task(**overrides)

    def test_event_requires_valid_duration_and_cannot_be_completed(self):
        with self.assertRaises(sqlite3.IntegrityError):
            self.task(kind='event')
        self.task(kind='event', start_time='09:00', end_date='2026-11-02', end_time='10:00')
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute('UPDATE calendar_entries SET completed = 1')
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("UPDATE calendar_entries SET end_time = '08:00'")

    def test_all_day_end_is_exclusive(self):
        with self.assertRaises(sqlite3.IntegrityError):
            self.task(kind='event', all_day=1, end_date='2026-11-02')
        self.task(kind='event', all_day=1, end_date='2026-11-03')

    def test_exception_identity_and_independent_completion_storage(self):
        self.task(recurrence_rule='FREQ=WEEKLY;BYDAY=MO')
        for identifier, original, completed in [('a', '2026-11-02', 1), ('b', '2026-11-09', 0)]:
            self.db.execute('INSERT INTO recurrence_exceptions (id, entry_id, original_start, exception_type, completed) VALUES (?, ?, ?, ?, ?)', (identifier, 'task', original, 'completion', completed))
        self.assertEqual(self.db.execute('SELECT completed FROM recurrence_exceptions ORDER BY original_start').fetchall(), [(1,), (0,)])
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO recurrence_exceptions (id, entry_id, original_start, exception_type) VALUES ('c', 'task', '2026-11-02', 'cancelled')")
        self.assertEqual(self.db.execute('SELECT count(*) FROM calendar_entries').fetchone()[0], 1)

    def test_delivery_unique_key_and_cascades(self):
        self.task()
        self.db.execute("INSERT INTO reminders (id, entry_id, kind, minutes_before) VALUES ('r', 'task', 'relative', 10)")
        self.db.execute("INSERT INTO notification_deliveries (id, reminder_id, occurrence_key, scheduled_at) VALUES ('d', 'r', 'task:2026-11-02', '2026-11-02T11:50:00Z')")
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO notification_deliveries (id, reminder_id, occurrence_key, scheduled_at) VALUES ('d2', 'r', 'task:2026-11-02', '2026-11-02T11:50:00Z')")
        self.db.execute("DELETE FROM calendar_entries WHERE id='task'")
        self.assertEqual(self.db.execute('SELECT count(*) FROM reminders').fetchone()[0], 0)
        self.assertEqual(self.db.execute('SELECT count(*) FROM notification_deliveries').fetchone()[0], 0)

    def test_invalid_json_and_reminders_are_rejected(self):
        self.task()
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO app_settings (key, value) VALUES ('invalid', 'not-json')")
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO reminders (id, entry_id, kind, minutes_before) VALUES ('r', 'task', 'relative', -5)")

    def test_category_deletion_preserves_entry(self):
        self.task(category_id='personal')
        self.db.execute("DELETE FROM categories WHERE id='personal'")
        self.assertIsNone(self.db.execute('SELECT category_id FROM calendar_entries').fetchone()[0])

    def test_transaction_rollback(self):
        self.db.execute('BEGIN')
        self.task()
        self.db.rollback()
        self.assertEqual(self.db.execute('SELECT count(*) FROM calendar_entries').fetchone()[0], 0)

    def test_reminder_exception_must_belong_to_same_entry(self):
        self.task('first')
        self.task('second')
        self.db.execute("INSERT INTO recurrence_exceptions (id, entry_id, original_start, exception_type) VALUES ('ex', 'first', '2026-11-02', 'cancelled')")
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("INSERT INTO reminders (id, entry_id, exception_id, kind, minutes_before) VALUES ('r', 'second', 'ex', 'relative', 10)")

    def test_last_view_upsert_persists_after_reopen(self):
        source = (ROOT / 'src/services/settings-persistence.ts').read_text(encoding='utf-8')
        match = re.search(r'const UPSERT = "([^"]+)";', source)
        self.assertIsNotNone(match)
        query = match.group(1)
        self.db.execute(query, {'1': 'last_view', '2': json.dumps('week')})
        self.db.execute(query, {'1': 'last_view', '2': json.dumps('year')})
        self.db.commit()
        self.db.close()
        self.db = sqlite3.connect(self.path)
        rows = self.db.execute("SELECT value FROM app_settings WHERE key = 'last_view'").fetchall()
        self.assertEqual(rows, [('"year"',)])

    def test_native_query_loads_anchors_without_materializing_future(self):
        source = (ROOT / 'src-tauri/calendar-core/src/repository.rs').read_text(encoding='utf-8')
        match = re.search(r'query_as\("(SELECT \* FROM calendar_entries ORDER BY start_date,id)"\)', source)
        self.assertIsNotNone(match)
        self.task('span', kind='event', all_day=1, start_date='2026-12-30', end_date='2027-01-03')
        self.task('overnight', start_date='2026-12-31', start_time='23:50')
        self.task('inside', start_date='2027-06-01')
        self.task('outside', start_date='2020-01-01')
        self.task('next-year', start_date='2028-01-01')
        self.task('recurring', start_date='2027-06-01', recurrence_rule='FREQ=DAILY')
        rows = self.db.execute(match.group(1)).fetchall()
        self.assertEqual({row[0] for row in rows}, {'span', 'overnight', 'inside', 'outside', 'next-year', 'recurring'})
        self.assertEqual(len(rows), 6)


if __name__ == '__main__':
    unittest.main(verbosity=2)
