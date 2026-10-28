import type { CalendarView } from './calendar';
export interface AppPreferences {
  calendar_text_color: string | null; calendar_background_color: string | null;
  calendar_font_size: number; calendar_bold: boolean; calendar_italic: boolean; calendar_underline: boolean;
  theme: 'light' | 'dark' | 'system'; first_day: 0 | 1; default_view: CalendarView; show_completed: boolean;
  notifications_enabled: boolean; default_reminder_minutes: number; autostart: boolean;
  start_in_tray: boolean; minimize_to_tray: boolean; close_behavior: 'exit' | 'tray';
}
export const DEFAULT_PREFERENCES: AppPreferences = {
  calendar_text_color: null, calendar_background_color: null,
  calendar_font_size: 12, calendar_bold: false, calendar_italic: false, calendar_underline: false,
  theme: 'system', first_day: 0, default_view: 'month', show_completed: true,
  notifications_enabled: true, default_reminder_minutes: 10, autostart: false,
  start_in_tray: false, minimize_to_tray: false, close_behavior: 'exit',
};
export interface BackupPreview { token: string; entries: number; categories: number; exceptions: number; exported_at: string }
export interface RuntimeStatus { last_check: string | null; last_error: string | null }
