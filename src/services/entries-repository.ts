import type { CalendarEntry } from '@/types/calendar';
import { invoke } from '@tauri-apps/api/core';

export async function listEntriesInRange(start: string, end: string): Promise<CalendarEntry[]> {
  return invoke<CalendarEntry[]>('list_occurrences', { start, end });
}
