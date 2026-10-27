import { addDays, format } from 'date-fns';
import { ptBR } from 'date-fns/locale';
import type { CalendarView } from '@/types/calendar';
import { startOfCivilWeek } from './dates';

export function timedIntervalLabel(start:Date,end:Date):string {
  const pattern=start.getTimezoneOffset()===end.getTimezoneOffset()?'dd/MM/yyyy HH:mm':'dd/MM/yyyy HH:mm xxx';
  return `${format(start,pattern,{locale:ptBR})} – ${format(end,pattern,{locale:ptBR})}`;
}

export function periodTitle(date: Date, view: CalendarView, firstDay: 0 | 1): string {
  const print = (value: Date, pattern: string) => format(value, pattern, { locale: ptBR });
  if (view === 'year') return print(date, 'yyyy');
  if (view === 'month') return print(date, "MMMM 'de' yyyy");
  const start = startOfCivilWeek(date, firstDay);
  const end = addDays(start, 6);
  if (start.getFullYear() !== end.getFullYear()) return `${print(start, 'd MMM yyyy')} – ${print(end, 'd MMM yyyy')}`;
  if (start.getMonth() !== end.getMonth()) return `${print(start, 'd MMM')} – ${print(end, 'd MMM yyyy')}`;
  return `${print(start, 'd')}–${print(end, "d 'de' MMMM 'de' yyyy")}`;
}
