import type { CalendarEntry, Category, EntryKind } from '../../types/calendar';
import { entryTypography } from './typography.ts';
import { dateKey, parseCivilDate, type DateRange } from './dates.ts';

export interface CalendarFilters {
  kinds: EntryKind[];
  hiddenCategoryIds: string[];
  showCompleted: boolean;
}

export function filterEntries(entries: CalendarEntry[], filters: CalendarFilters): CalendarEntry[] {
  return entries.filter((entry) => filters.kinds.includes(entry.kind)
    && (entry.category_id === null || !filters.hiddenCategoryIds.includes(entry.category_id))
    && (filters.showCompleted || entry.kind !== 'task' || entry.completed === 0));
}

export function entryInterval(entry: CalendarEntry): DateRange & { allDay: boolean } {
  const allDay = entry.all_day === 1 || entry.start_time === null;
  if (!allDay && entry.start_utc && entry.end_utc) return { start: new Date(entry.start_utc), end: new Date(entry.end_utc), allDay: false };
  const start = parseCivilDate(entry.start_date, allDay ? '00:00' : entry.start_time ?? '00:00');
  let end: Date;
  if (allDay) {
    end = entry.end_date && entry.end_date > entry.start_date
      ? parseCivilDate(entry.end_date)
      : new Date(start.getFullYear(), start.getMonth(), start.getDate() + 1);
  } else {
    end = entry.end_date && entry.end_time
      ? parseCivilDate(entry.end_date, entry.end_time)
      : new Date(start.getTime() + 30 * 60_000);
  }
  return { start, end, allDay };
}

export function entriesOnDay(entries: CalendarEntry[], date: Date): CalendarEntry[] {
  const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const end = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1);
  return entries.filter((entry) => {
    const interval = entryInterval(entry);
    return interval.start < end && interval.end > start;
  });
}

export interface DaySummary { tasks: number; events: number }

// Somente os dias do intervalo visível: nunca expandir eventos longos fora dele.
export function buildDaySummaries(entries: CalendarEntry[], range: DateRange): Map<string, DaySummary> {
  const summaries = new Map<string, DaySummary>();
  for (const entry of entries) {
    const interval = entryInterval(entry);
    const start = new Date(Math.max(range.start.getTime(), interval.start.getTime()));
    const day = new Date(start.getFullYear(), start.getMonth(), start.getDate());
    const end = Math.min(range.end.getTime(), interval.end.getTime());
    while (day.getTime() < end) {
      const key = dateKey(day);
      const value = summaries.get(key) ?? { tasks: 0, events: 0 };
      if (entry.kind === 'task') value.tasks++;
      else value.events++;
      summaries.set(key, value);
      day.setDate(day.getDate() + 1);
    }
  }
  return summaries;
}

export function contrastingText(color: string): '#ffffff' | '#172033' {
  const luminance = (hex: string) => {
    const channels = [1, 3, 5].map((offset) => {
      const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
      return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    });
    return (channels[0] ?? 0) * 0.2126 + (channels[1] ?? 0) * 0.7152 + (channels[2] ?? 0) * 0.0722;
  };
  const background = luminance(color);
  const whiteContrast = 1.05 / (background + 0.05);
  const darkContrast = (background + 0.05) / (luminance('#172033') + 0.05);
  return whiteContrast >= darkContrast ? '#ffffff' : '#172033';
}

export function calendarEvent(entry: CalendarEntry, categories: Category[]) {
  const color = categories.find((category) => category.id === entry.category_id)?.color ?? '#1A73E8';
  const interval = entryInterval(entry);
  return {
    id: entry.id, title: entry.title, ...interval,
    backgroundColor: color, borderColor: color, textColor: contrastingText(color),
    classNames: entry.completed === 1 ? ['entry-completed'] : [],
    extendedProps: { typography: entryTypography(entry), kind: entry.kind, completed: entry.completed === 1 },
  };
}
