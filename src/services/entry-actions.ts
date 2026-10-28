import { movedEntryDraft } from '@/features/calendar/move';
import { invoke } from '@tauri-apps/api/core';
import type { CalendarEntry, EditScope, EntryDraft, ReminderDraft } from '@/types/calendar';
import { entryIdentity } from '@/features/entries/form';
import { useCalendarStore } from '@/stores/calendar-store';

export async function saveEntry(entry: CalendarEntry | undefined, draft: EntryDraft, scope: EditScope) {
  const identity = entry ? entryIdentity(entry) : { id: null, original: null };
  await invoke('save_entry', { ...identity, draft, scope });
  useCalendarStore.getState().touch();
}
export async function getEntryDetails(entry: CalendarEntry) {
  return invoke<{ occurrence: CalendarEntry; reminders: ReminderDraft[] }>('entry_details', entryIdentity(entry));
}
export async function removeEntry(entry: CalendarEntry, scope: EditScope) {
  await invoke('delete_entry', { ...entryIdentity(entry), scope }); useCalendarStore.getState().touch();
}
export async function duplicateEntry(entry: CalendarEntry) {
  await invoke('duplicate_entry', entryIdentity(entry)); useCalendarStore.getState().touch();
}
export async function completeEntry(entry: CalendarEntry) {
  await invoke('complete_entry', { ...entryIdentity(entry), completed: entry.completed === 0 }); useCalendarStore.getState().touch();
}

export async function moveEntry(entry: CalendarEntry, start: Date, end: Date | null, allDay: boolean) {
  const details = await getEntryDetails(entry);
  await saveEntry(entry, movedEntryDraft(details.occurrence, details.reminders, start, end, allDay), 'one');
}
