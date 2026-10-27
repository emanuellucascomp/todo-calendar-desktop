// Datas civis: nunca analisar YYYY-MM-DD com new Date(string), que usa UTC.
export function dateKey(date: Date): string {
  const year = date.getFullYear().toString().padStart(4, '0');
  const month = (date.getMonth() + 1).toString().padStart(2, '0');
  const day = date.getDate().toString().padStart(2, '0');
  return `${year}-${month}-${day}`;
}

export function monthCells(month: Date, firstDay: 0 | 1): Date[] {
  // Meio-dia evita transições históricas de horário de verão à meia-noite.
  const first = new Date(month.getFullYear(), month.getMonth(), 1, 12);
  const offset = (first.getDay() - firstDay + 7) % 7;
  return Array.from({ length: 42 }, (_, index) =>
    new Date(first.getFullYear(), first.getMonth(), 1 - offset + index, 12),
  );
}

export function shiftMonth(date: Date, amount: number): Date {
  const first = new Date(date.getFullYear(), date.getMonth() + amount, 1, 12);
  const last = new Date(first.getFullYear(), first.getMonth() + 1, 0, 12).getDate();
  return new Date(first.getFullYear(), first.getMonth(), Math.min(date.getDate(), last), 12);
}
import type { CalendarView } from '../../types/calendar';


export function startOfCivilWeek(date: Date, firstDay: 0 | 1): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() - (date.getDay() - firstDay + 7) % 7, 12);
}

export function navigatePeriod(date: Date, view: CalendarView, direction: -1 | 1): Date {
  if (view === 'month') return shiftMonth(date, direction);
  if (view === 'year') return shiftMonth(date, direction * 12);
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + direction * 7, 12);
}

export interface DateRange { start: Date; end: Date }

// O término de todos os intervalos é exclusivo, como no FullCalendar.
export function calendarRange(date: Date, view: CalendarView, firstDay: 0 | 1): DateRange {
  if (view === 'year') return {
    start: new Date(date.getFullYear(), 0, 1), end: new Date(date.getFullYear() + 1, 0, 1),
  };
  const anchor = view === 'week' ? date : new Date(date.getFullYear(), date.getMonth(), 1, 12);
  const first = startOfCivilWeek(anchor, firstDay);
  return {
    start: new Date(first.getFullYear(), first.getMonth(), first.getDate()),
    end: new Date(first.getFullYear(), first.getMonth(), first.getDate() + (view === 'week' ? 7 : 42)),
  };
}

export function parseCivilDate(value: string, time = '00:00'): Date {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value) || !/^([01]\d|2[0-3]):[0-5]\d$/.test(time)) {
    throw new Error('Data ou horário inválido no calendário.');
  }
  const year = Number(value.slice(0, 4));
  const month = Number(value.slice(5, 7)) - 1;
  const day = Number(value.slice(8, 10));
  const date = new Date(0);
  date.setFullYear(year, month, day);
  date.setHours(Number(time.slice(0, 2)), Number(time.slice(3, 5)), 0, 0);
  if (dateKey(date) !== value) throw new Error('Data inválida no calendário.');
  return date;
}
