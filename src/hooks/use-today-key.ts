import { useEffect, useState } from 'react';
import { dateKey } from '@/features/calendar/dates';

export function useTodayKey() {
  const [today, setToday] = useState(() => dateKey(new Date()));
  useEffect(() => {
    const refresh = () => setToday(dateKey(new Date()));
    const timer = window.setInterval(refresh, 30_000);
    document.addEventListener('visibilitychange', refresh);
    window.addEventListener('focus', refresh);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', refresh);
      window.removeEventListener('focus', refresh);
    };
  }, []);
  return today;
}
