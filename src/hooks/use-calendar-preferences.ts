import { useEffect, useState } from 'react';
import { useCalendarStore } from '@/stores/calendar-store';
import { saveLastView } from '@/services/settings-repository';

export function useCalendarPreferences(ready: boolean) {
  const view = useCalendarStore((state) => state.view);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    if (!ready) return;
    let active = true;
    setError(null);
    void saveLastView(view).catch(() => {
      if (active) setError('Não foi possível salvar a visualização escolhida.');
    });
    return () => { active = false; };
  }, [ready, view, attempt]);

  return { error, retry: () => setAttempt((value) => value + 1) };
}
