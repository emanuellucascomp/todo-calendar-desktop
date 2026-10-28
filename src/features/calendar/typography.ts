import type { CalendarEntry } from '../../types/calendar';

export function entryTypography(entry: Pick<CalendarEntry, 'font_size' | 'bold' | 'italic' | 'underline' | 'completed'>) {
  return {
    fontSize: `${entry.font_size ?? 12}px`,
    fontWeight: entry.bold ? 700 : 400,
    fontStyle: entry.italic ? 'italic' as const : 'normal' as const,
    textDecoration: [entry.underline && 'underline', entry.completed === 1 && 'line-through'].filter(Boolean).join(' ') || 'none',
  };
}
