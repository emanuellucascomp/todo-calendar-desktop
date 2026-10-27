import { useMemo } from 'react';
import { calendarRange } from '@/features/calendar/dates';
import { filterEntries } from '@/features/calendar/entries';
import { useCalendarStore } from '@/stores/calendar-store';
import { useCalendarEntries } from './use-calendar-entries';

export function useCalendarData(ready: boolean, firstDay: 0 | 1, showCompleted: boolean) {
  const date = useCalendarStore((state) => state.selectedDate);
  const view = useCalendarStore((state) => state.view);
  const kinds = useCalendarStore((state) => state.visibleKinds);
  const hiddenCategoryIds = useCalendarStore((state) => state.hiddenCategoryIds);
  const range = useMemo(() => calendarRange(date, view, firstDay), [date, view, firstDay]);
  const query = useCalendarEntries(range, ready);
  const entries = useMemo(() => filterEntries(query.entries, { kinds, hiddenCategoryIds, showCompleted }),
    [query.entries, kinds, hiddenCategoryIds, showCompleted]);
  return { ...query, entries };
}
