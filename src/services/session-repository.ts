import { z } from 'zod';
import { getDatabase } from './database';
import { dateKey, parseCivilDate } from '@/features/calendar/dates';
import { useCalendarStore } from '@/stores/calendar-store';

const sessionSchema = z.object({
  view: z.enum(['month', 'week', 'week-grid', 'year']).optional(),
  selectedDate: z.string().refine((value) => { try { parseCivilDate(value); return true; } catch { return false; } }),
  sidebarOpen: z.boolean(), minimalMode: z.boolean(),
  visibleKinds: z.array(z.enum(['task', 'event'])), hiddenCategoryIds: z.array(z.string()),
});
let pending: Promise<unknown> = Promise.resolve();
export async function restoreSession() {
  const db = await getDatabase();
  const rows = await db.select<{ value: string }[]>("SELECT value FROM app_settings WHERE key = $1", ['calendar_session']);
  if (!rows[0]) return;
  const session = sessionSchema.parse(JSON.parse(rows[0].value));
  const { view, ...rest } = session;
  useCalendarStore.setState({ ...rest, ...(view ? { view } : {}), selectedDate: parseCivilDate(session.selectedDate) });
}
export function saveSession() {
  const state = useCalendarStore.getState();
  const value = JSON.stringify(sessionSchema.parse({ ...state, selectedDate: dateKey(state.selectedDate) }));
  const write = pending.catch(() => undefined).then(async () => {
    const db = await getDatabase();
    await db.execute("INSERT INTO app_settings(key,value) VALUES($1,$2) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')", ['calendar_session', value]);
  });
  pending = write;
  return write;
}
export async function flushSession() { await pending; }
