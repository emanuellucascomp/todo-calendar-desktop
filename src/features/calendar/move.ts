import type { CalendarEntry, EntryDraft, ReminderDraft } from '../../types/calendar';
import { dateKey } from './dates.ts';
import { entryInterval } from './entries.ts';

// Horários persistidos pertencem ao fuso da série; datas de dia inteiro são civis locais.
export function movedEntryDraft(entry: CalendarEntry, reminders: ReminderDraft[], start: Date, end: Date | null, allDay: boolean): EntryDraft {
  const previous = entryInterval(entry);
  const finish = allDay ? end ?? new Date(start.getFullYear(), start.getMonth(), start.getDate() + 1)
    : new Date(start.getTime() + (previous.allDay ? (end?.getTime() ?? start.getTime() + 1_800_000) - start.getTime() : previous.end.getTime() - previous.start.getTime()));
  const civil = (instant: Date) => {
    const parts = new Intl.DateTimeFormat('en-CA', { timeZone: entry.timezone, year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }).formatToParts(instant);
    const value = (type: string) => parts.find((part) => part.type === type)?.value ?? '';
    return { date: `${value('year')}-${value('month')}-${value('day')}`, time: `${value('hour')}:${value('minute')}` };
  };
  const first = allDay ? null : civil(start);
  const last = allDay ? null : civil(finish);
  return {
    font_size: entry.font_size ?? 12, bold: entry.bold ?? false, italic: entry.italic ?? false, underline: entry.underline ?? false,
    kind: entry.kind, title: entry.title, description: entry.description,
    start_date: first?.date ?? dateKey(start), start_time: first?.time ?? null,
    end_date: last?.date ?? dateKey(finish), end_time: last?.time ?? null,
    all_day: allDay, timezone: entry.timezone, category_id: entry.category_id, priority: entry.priority,
    recurrence_rule: null, reminders,
    start_utc: allDay ? null : start.toISOString(), end_utc: allDay ? null : finish.toISOString(),
    duration_seconds: allDay ? null : (finish.getTime() - start.getTime()) / 1000,
  };
}
