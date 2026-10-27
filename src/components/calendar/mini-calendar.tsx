import { useEffect, useMemo, useState } from 'react';
import { format, isSameDay, isSameMonth } from 'date-fns';
import { ptBR } from 'date-fns/locale';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { monthCells, dateKey, shiftMonth } from '@/features/calendar/dates';
import { useCalendarStore } from '@/stores/calendar-store';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { useTodayKey } from '@/hooks/use-today-key';

export function MiniCalendar({ firstDay }: { firstDay: 0 | 1 }) {
  const today = useTodayKey();
  const selected = useCalendarStore((state) => state.selectedDate);
  const selectDate = useCalendarStore((state) => state.selectDate);
  const [cursor, setCursor] = useState(selected);
  useEffect(() => { setCursor(selected); }, [selected]);
  const days = useMemo(() => monthCells(cursor, firstDay), [cursor, firstDay]);
  const weekDays = firstDay === 0 ? ['D', 'S', 'T', 'Q', 'Q', 'S', 'S'] : ['S', 'T', 'Q', 'Q', 'S', 'S', 'D'];

  return (
    <section aria-label="Navegação por datas" className="mini-calendar">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-medium capitalize">{format(cursor, 'MMMM yyyy', { locale: ptBR })}</h2>
        <div className="flex">
          <Button variant="ghost" size="sm" className="size-7 p-0" aria-label="Mês anterior no mini calendário" onClick={() => setCursor((date) => shiftMonth(date, -1))}><ChevronLeft /></Button>
          <Button variant="ghost" size="sm" className="size-7 p-0" aria-label="Próximo mês no mini calendário" onClick={() => setCursor((date) => shiftMonth(date, 1))}><ChevronRight /></Button>
        </div>
      </div>
      <div className="grid grid-cols-7 text-center">
        {weekDays.map((day, index) => <span className="py-1 text-[11px] text-muted-foreground" key={index} aria-hidden="true">{day}</span>)}
        {days.map((day) => (
          <button type="button" key={dateKey(day)}
            aria-label={format(day, "EEEE, d 'de' MMMM 'de' yyyy", { locale: ptBR })}
            aria-pressed={isSameDay(day, selected)} aria-current={dateKey(day) === today ? 'date' : undefined}
            onClick={() => selectDate(day)}
            className={cn('mx-auto flex size-7 items-center justify-center rounded-full text-[11px] transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-primary',
              !isSameMonth(day, cursor) && 'text-muted-foreground/60',
              dateKey(day) === today && 'bg-primary text-white hover:bg-primary/90',
              isSameDay(day, selected) && dateKey(day) !== today && 'bg-blue-100 text-blue-800')}
          >{day.getDate()}</button>
        ))}
      </div>
    </section>
  );
}
