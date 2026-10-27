export type EntryKind = 'task' | 'event';
export type CalendarView = 'week' | 'week-grid' | 'month' | 'year';
export type Priority = 'low' | 'medium' | 'high';

export interface Category {
  id: string;
  name: string;
  color: string;
  created_at: string;
  updated_at: string;
}

// Correspondência explícita com SQLite; a expansão de recorrências será um modelo separado.
export interface CalendarEntry {
  id: string;
  kind: EntryKind;
  title: string;
  description: string;
  start_date: string;
  start_time: string | null;
  end_date: string | null;
  end_time: string | null;
  all_day: 0 | 1;
  timezone: string;
  category_id: string | null;
  priority: Priority;
  completed: 0 | 1;
  recurrence_rule: string | null;
  series_parent_id: string | null;
  created_at: string;
  updated_at: string;
  series_id?: string;
  occurrence_id?: string;
  original_start?: string;
  series_end_before?: string | null;
  start_utc?: string | null;
  end_utc?: string | null;
  duration_seconds?: number | null;
}

export type EditScope = 'one' | 'future' | 'series';
export interface ReminderDraft { id?: string | null; kind: 'relative' | 'absolute'; minutes_before: number | null; scheduled_at: string | null }
export interface EntryDraft {
  kind: EntryKind; title: string; description: string; start_date: string; start_time: string | null;
  end_date: string | null; end_time: string | null; all_day: boolean; timezone: string;
  category_id: string | null; priority: Priority; recurrence_rule: string | null; reminders: ReminderDraft[];
  start_utc?: string | null; end_utc?: string | null;
  duration_seconds?: number | null;
}
export interface EditorRequest { entry?: CalendarEntry; start?: Date; end?: Date; allDay?: boolean }

export interface FoundationSettings {
  firstDay: 0 | 1;
  showCompleted: boolean;
  lastView: CalendarView;
}

export interface DatabaseInfo { path: string }
