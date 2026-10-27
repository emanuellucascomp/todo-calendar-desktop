import { useEffect, useState } from 'react';
import { listEntriesInRange } from '@/services/entries-repository';
import { dateKey, type DateRange } from '@/features/calendar/dates';
import type { CalendarEntry } from '@/types/calendar';
import { useCalendarStore } from '@/stores/calendar-store';

interface Result { key: string; entries: CalendarEntry[]; error: string | null }
const EMPTY_ENTRIES: CalendarEntry[] = [];

export function useCalendarEntries(range: DateRange, ready: boolean) {
  const revision = useCalendarStore((state) => state.revision);
  const start = dateKey(range.start);
  const end = dateKey(range.end);
  const key = `${start}/${end}`;
  const [result, setResult] = useState<Result | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    const refresh = () => setAttempt((value) => value + 1);
    window.addEventListener('focus', refresh);
    return () => window.removeEventListener('focus', refresh);
  }, []);

  useEffect(() => {
    if (!ready) return;
    let active = true;
    const requestKey = `${start}/${end}`;
    void listEntriesInRange(start, end).then((entries) => {
      if (active) setResult({ key: requestKey, entries, error: null });
    }).catch((error: unknown) => {
      if (active) setResult({ key: requestKey, entries: [], error: `Não foi possível carregar os registros: ${String(error)}` });
    });
    return () => { active = false; };
  }, [start, end, ready, attempt, revision]);

  const current = ready && result?.key === key ? result : null;
  return {
    entries: current?.entries ?? EMPTY_ENTRIES,
    loading: ready && current === null,
    error: current?.error ?? null,
    retry: () => setAttempt((value) => value + 1),
  };
}
