import type { CalendarView } from './calendar';
export interface AppPreferences {
  theme: 'light' | 'dark' | 'system'; first_day: 0 | 1; default_view: CalendarView; show_completed: boolean;
  notifications_enabled: boolean; default_reminder_minutes: number; autostart: boolean;
  minimize_to_tray: boolean; close_behavior: 'exit' | 'tray';
}
export const DEFAULT_PREFERENCES: AppPreferences = {
  theme: 'system', first_day: 0, default_view: 'month', show_completed: true,
  notifications_enabled: true, default_reminder_minutes: 10, autostart: false,
  minimize_to_tray: false, close_behavior: 'exit',
};
export interface BackupPreview { token: string; entries: number; categories: number; exceptions: number; exported_at: string }
export interface RuntimeStatus { last_check: string | null; last_error: string | null }
