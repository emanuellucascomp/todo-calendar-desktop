import { useEffect, useMemo, useRef } from 'react';
import FullCalendar from '@fullcalendar/react';
import dayGridPlugin from '@fullcalendar/daygrid';
import timeGridPlugin from '@fullcalendar/timegrid';
import interactionPlugin from '@fullcalendar/interaction';
import ptBrLocale from '@fullcalendar/core/locales/pt-br';
import type { CalendarEntry, Category } from '@/types/calendar';
import { useCalendarStore } from '@/stores/calendar-store';
import { calendarEvent } from '@/features/calendar/entries';
import type { CalendarSelection } from './selection-dialog';

interface Props {
  firstDay: 0 | 1;
  entries: CalendarEntry[];
  categories: Category[];
  onSelect: (selection: CalendarSelection) => void;
  onToggle: (entry: CalendarEntry) => void;
  saving: boolean;
}

export function ScheduleCalendar({ firstDay, entries, categories, onSelect, onToggle, saving }: Props) {
  const ref = useRef<FullCalendar>(null);
  const container = useRef<HTMLDivElement>(null);
  const date = useCalendarStore((state) => state.selectedDate);
  const view = useCalendarStore((state) => state.view);
  const selectDate = useCalendarStore((state) => state.selectDate);
  const type = view === 'week' ? 'timeGridWeek' : view === 'week-grid' ? 'dayGridWeek' : 'dayGridMonth';
  const events = useMemo(() => entries.map((entry) => calendarEvent(entry, categories)), [entries, categories]);

  useEffect(() => {
    const api = ref.current?.getApi();
    if (api?.view.type !== type) api?.changeView(type, date);
    else api?.gotoDate(date);
  }, [date, type]);

  useEffect(() => {
    if (!container.current) return;
    let frame = 0;
    let lastSize = '';
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const size = `${entry.contentRect.width}/${entry.contentRect.height}`;
      if (size === lastSize) return;
      lastSize = size;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => ref.current?.getApi().updateSize());
    });
    observer.observe(container.current);
    return () => { observer.disconnect(); cancelAnimationFrame(frame); };
  }, []);

  function openDay(day: Date, allDay: boolean) {
    const end = allDay
      ? new Date(day.getFullYear(), day.getMonth(), day.getDate() + 1)
      : new Date(day.getTime() + 30 * 60_000);
    selectDate(day);
    onSelect({ start: day, end, allDay });
  }

  return <div ref={container} className="schedule-calendar"><FullCalendar ref={ref}
    plugins={[dayGridPlugin, timeGridPlugin, interactionPlugin]} initialView={type}
    initialDate={date} locale={ptBrLocale} firstDay={firstDay} timeZone="local"
    headerToolbar={false} height="100%" showNonCurrentDates
    nowIndicator allDaySlot allDayText="Dia inteiro" weekends
    slotDuration="00:30:00" slotLabelInterval="01:00:00" scrollTime="07:00:00" scrollTimeReset={false}
    slotLabelFormat={{ hour: '2-digit', minute: '2-digit', hour12: false }}
    eventTimeFormat={{ hour: '2-digit', minute: '2-digit', hour12: false }}
    displayEventEnd eventDisplay="block" events={events} editable={false} eventInteractive
    views={{
      dayGridMonth: { fixedWeekCount: true, dayHeaderFormat: { weekday: 'short' }, dayMaxEvents: 3 },
      dayGridWeek: { fixedWeekCount: false, dayHeaderFormat: { weekday: 'long' }, dayMaxEvents: true },
      timeGridWeek: { dayHeaderFormat: { weekday: 'short', day: '2-digit', month: '2-digit' }, dayMaxEvents: false },
    }}
    moreLinkClick="popover" moreLinkText={(count) => `+${count} mais`}
    navLinks navLinkDayClick={(day) => openDay(day, true)}
    dateClick={(info) => openDay(info.date, info.allDay)}
    selectable selectMirror selectMinDistance={5}
    select={(info) => {
      selectDate(info.start);
      onSelect({ start: info.start, end: info.end, allDay: info.allDay });
      ref.current?.getApi().unselect();
    }}
    eventClick={(info) => {
      info.jsEvent.preventDefault();
      if (info.event.start && info.event.end) onSelect({
        start: info.event.start, end: info.event.end, allDay: info.event.allDay, entryId: info.event.id,
      });
    }}
    eventContent={(info) => <span className="calendar-entry-content">
      {info.event.extendedProps.kind === 'task' && <input type="checkbox" checked={Boolean(info.event.extendedProps.completed)} disabled={saving}
        aria-label={`${info.event.extendedProps.completed ? 'Reabrir' : 'Concluir'} ${info.event.title}`}
        onClick={(event) => event.stopPropagation()} onMouseDown={(event) => event.stopPropagation()} onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.stopPropagation()}
        onChange={() => { const entry = entries.find((entry) => entry.id === info.event.id); if (entry) onToggle(entry); }} />}
      {info.timeText && <span className="calendar-entry-time">{info.timeText}</span>}
      <span className="calendar-entry-title">{info.event.title}</span>
    </span>}
    eventDidMount={(info) => {
      info.el.setAttribute('aria-label', `${info.event.title}, ${info.event.extendedProps.kind === 'task'
        ? info.event.extendedProps.completed ? 'tarefa concluída' : 'tarefa pendente' : 'evento'}`);
    }}
  /></div>;
}
