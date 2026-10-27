import { memo, useMemo } from 'react';
import { format } from 'date-fns';
import { ptBR } from 'date-fns/locale';
import { dateKey, monthCells } from '@/features/calendar/dates';
import { buildDaySummaries, type DaySummary } from '@/features/calendar/entries';
import type { CalendarEntry } from '@/types/calendar';
import { useCalendarStore } from '@/stores/calendar-store';
import { useTodayKey } from '@/hooks/use-today-key';
import { cn } from '@/lib/utils';

interface MonthProps {
  year: number;
  month: number;
  firstDay: 0 | 1;
  selectedKey: string | null;
  todayKey: string;
  summaries: Map<string, DaySummary>;
  onDay: (day: Date) => void;
}

const YearMonth = memo(function YearMonth({ year, month, firstDay, selectedKey, todayKey, summaries, onDay }: MonthProps) {
  const days = useMemo(() => monthCells(new Date(year, month, 1), firstDay), [year, month, firstDay]);
  const weekdays = firstDay === 0 ? ['D', 'S', 'T', 'Q', 'Q', 'S', 'S'] : ['S', 'T', 'Q', 'Q', 'S', 'S', 'D'];
  const name = format(new Date(year, month, 1), 'MMMM', { locale: ptBR });
  return <section className="year-month" aria-label={`${name} de ${year}`}>
    <h2>{name}</h2>
    <div className="year-month-grid">
      {weekdays.map((day, index) => <span key={`weekday-${index}`} className="year-weekday" aria-hidden="true">{day}</span>)}
      {days.map((day) => {
        const key = dateKey(day);
        if (day.getMonth() !== month) return <span key={key} aria-hidden="true" />;
        const summary = summaries.get(key);
        const description = summary ? `, ${summary.tasks} tarefa(s), ${summary.events} evento(s)` : ', sem registros';
        return <button type="button" key={key}
          onClick={() => onDay(day)} title={`Abrir semana${description}`}
          aria-label={`${format(day, "EEEE, d 'de' MMMM 'de' yyyy", { locale: ptBR })}${description}. Abrir semana.`}
          aria-current={key === todayKey ? 'date' : undefined}
          className={cn('year-day', key === todayKey && 'year-day-today', key === selectedKey && 'year-day-selected')}>
          <span>{day.getDate()}</span>
          <span className="year-markers" aria-hidden="true">
            {!!summary?.tasks && <i className="year-task-marker" />}
            {!!summary?.events && <i className="year-event-marker" />}
          </span>
        </button>;
      })}
    </div>
  </section>;
});

export function YearCalendar({ entries, firstDay }: { entries: CalendarEntry[]; firstDay: 0 | 1 }) {
  const date = useCalendarStore((state) => state.selectedDate);
  const openWeek = useCalendarStore((state) => state.openWeek);
  const todayKey = useTodayKey();
  const year = date.getFullYear();
  const summaries = useMemo(() => buildDaySummaries(entries, {
    start: new Date(year, 0, 1), end: new Date(year + 1, 0, 1),
  }), [entries, year]);
  const selectedKey = dateKey(date);

  return <div className="year-calendar">
    <div className="year-legend"><span><i className="year-task-marker" /> Tarefa</span><span><i className="year-event-marker" /> Evento</span><span>Clique em um dia para abrir a semana</span></div>
    <div className="year-grid">{Array.from({ length: 12 }, (_, month) => <YearMonth key={month}
      year={year} month={month} firstDay={firstDay} todayKey={todayKey}
      selectedKey={date.getMonth() === month ? selectedKey : null}
      summaries={summaries} onDay={openWeek}
    />)}</div>
  </div>;
}
