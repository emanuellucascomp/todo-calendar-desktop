import assert from 'node:assert/strict';
import { calendarRange, dateKey, navigatePeriod, parseCivilDate } from '../src/features/calendar/dates.ts';
import { buildDaySummaries, calendarEvent, entriesOnDay, filterEntries } from '../src/features/calendar/entries.ts';
import type { CalendarEntry } from '../src/types/calendar';

export function entry(overrides: Partial<CalendarEntry> = {}): CalendarEntry {
  return {
    id: 'entry', kind: 'task', title: 'Tarefa de teste', description: '',
    start_date: '2026-11-02', start_time: null, end_date: null, end_time: null,
    all_day: 0, timezone: 'America/Sao_Paulo', category_id: 'personal', priority: 'medium',
    completed: 0, recurrence_rule: null, series_parent_id: null,
    created_at: '2026-10-26T12:00:00Z', updated_at: '2026-10-26T12:00:00Z', ...overrides,
  };
}

export const calendarCases: { name: string; run: () => void }[] = [
  { name: 'semana em grade usa os mesmos sete dias da semana com horários', run() {
    for (const firstDay of [0, 1] as const) {
      for (const date of [new Date(2026, 11, 31), new Date(2026, 10, 1), new Date(2024, 1, 29)]) {
        const grid = calendarRange(date, 'week-grid', firstDay);
        assert.deepEqual(grid, calendarRange(date, 'week', firstDay));
        assert.equal(grid.start.getDay(), firstDay);
        for (const direction of [-1, 1] as const) {
          assert.equal(dateKey(navigatePeriod(date, 'week-grid', direction)), dateKey(navigatePeriod(date, 'week', direction)));
        }
      }
    }
  } },
  { name: 'navega uma semana incluindo mudança de mês e ano', run() {
    assert.equal(dateKey(navigatePeriod(new Date(2026, 11, 29), 'week', 1)), '2027-01-05');
    assert.equal(dateKey(navigatePeriod(new Date(2027, 0, 3), 'week', -1)), '2026-12-27');
  } },
  { name: 'navegação anual limita 29 de fevereiro a 28', run() {
    assert.equal(dateKey(navigatePeriod(new Date(2024, 1, 29), 'year', 1)), '2025-02-28');
    assert.equal(dateKey(navigatePeriod(new Date(2024, 1, 29), 'year', -1)), '2023-02-28');
  } },
  { name: 'semana de domingo cruza o ano e tem término exclusivo', run() {
    const range = calendarRange(new Date(2026, 0, 1), 'week', 0);
    assert.equal(dateKey(range.start), '2025-12-28');
    assert.equal(dateKey(range.end), '2026-01-04');
  } },
  { name: 'semana respeita segunda-feira como primeiro dia', run() {
    const range = calendarRange(new Date(2026, 10, 1), 'week', 1);
    assert.equal(dateKey(range.start), '2026-10-26');
    assert.equal(dateKey(range.end), '2026-11-02');
  } },
  { name: 'intervalo mensal inclui as 42 células e ano inclui só o ano', run() {
    const month = calendarRange(new Date(2026, 1, 15), 'month', 0);
    assert.equal(dateKey(month.start), '2026-02-01');
    assert.equal(dateKey(month.end), '2026-03-15');
    const year = calendarRange(new Date(2024, 6, 20), 'year', 1);
    assert.equal(dateKey(year.start), '2024-01-01');
    assert.equal(dateKey(year.end), '2025-01-01');
  } },
  { name: 'datas civis rejeitam dias inexistentes e horários inválidos', run() {
    for (const value of ['2026-02-29', '2026-02-30', '2026-13-01', '02/11/2026']) assert.throws(() => parseCivilDate(value));
    assert.throws(() => parseCivilDate('2026-11-02', '24:00'));
    assert.equal(dateKey(parseCivilDate('2024-02-29', '23:59')), '2024-02-29');
  } },
  { name: 'filtros de tipo, categoria e conclusão usam o mesmo conjunto', run() {
    const items = [entry(), entry({ id: 'done', completed: 1 }), entry({ id: 'event', kind: 'event', category_id: 'work' })];
    assert.deepEqual(filterEntries(items, { kinds: ['task', 'event'], hiddenCategoryIds: [], showCompleted: false }).map((item) => item.id), ['entry', 'event']);
    assert.deepEqual(filterEntries(items, { kinds: ['event'], hiddenCategoryIds: ['work'], showCompleted: true }), []);
    assert.equal(filterEntries([entry({ category_id: null })], { kinds: ['task'], hiddenCategoryIds: ['personal'], showCompleted: true }).length, 1);
  } },
  { name: 'evento de dia inteiro termina antes da data exclusiva', run() {
    const item = entry({ kind: 'event', all_day: 1, start_date: '2026-12-31', end_date: '2027-01-02' });
    assert.equal(entriesOnDay([item], new Date(2027, 0, 1)).length, 1);
    assert.equal(entriesOnDay([item], new Date(2027, 0, 2)).length, 0);
    const counts = buildDaySummaries([item], calendarRange(new Date(2027, 0, 1), 'year', 0));
    assert.deepEqual([...counts.keys()], ['2027-01-01']);
  } },
  { name: 'evento encerrado à meia-noite não marca o dia seguinte', run() {
    const item = entry({ kind: 'event', start_time: '23:00', end_date: '2026-11-03', end_time: '00:00' });
    assert.equal(entriesOnDay([item], new Date(2026, 10, 2)).length, 1);
    assert.equal(entriesOnDay([item], new Date(2026, 10, 3)).length, 0);
  } },
  { name: 'tarefa sem horário ocupa o dia e tarefa com horário tem duração visual de 30 minutos', run() {
    const untimed = calendarEvent(entry(), []);
    assert.equal(untimed.allDay, true);
    assert.equal(dateKey(untimed.end), '2026-11-03');
    const timed = calendarEvent(entry({ start_time: '23:50' }), []);
    assert.equal(timed.allDay, false);
    assert.equal(timed.end.getTime() - timed.start.getTime(), 30 * 60_000);
    assert.equal(entriesOnDay([entry({ start_time: '23:50' })], new Date(2026, 10, 3)).length, 1);
  } },
  { name: 'ano bissexto inclui 29 de fevereiro e indicadores somam tipos', run() {
    const items = [entry({ start_date: '2024-02-29' }), entry({ kind: 'event', start_date: '2024-02-29', all_day: 1, end_date: '2024-03-01' })];
    const counts = buildDaySummaries(items, calendarRange(new Date(2024, 0, 1), 'year', 0));
    assert.deepEqual(counts.get('2024-02-29'), { tasks: 1, events: 1 });
    assert.equal(counts.has('2024-03-01'), false);
  } },
  { name: 'evento com décadas de duração só gera indicadores no ano visível', run() {
    const item = entry({ kind: 'event', all_day: 1, start_date: '2000-01-01', end_date: '2099-01-01' });
    const counts = buildDaySummaries([item], calendarRange(new Date(2024, 0, 1), 'year', 0));
    assert.equal(counts.size, 366);
    assert.equal(counts.has('2023-12-31'), false);
    assert.equal(counts.has('2025-01-01'), false);
  } },
  { name: 'cor, identidade e conclusão são preservadas no modelo FullCalendar', run() {
    const event = calendarEvent(entry({ completed: 1 }), [{ id: 'personal', name: 'Pessoal', color: '#FFFF00', created_at: '', updated_at: '' }]);
    assert.equal(event.id, 'entry');
    assert.equal(event.backgroundColor, '#FFFF00');
    assert.equal(event.textColor, '#172033');
    assert.equal(event.extendedProps.completed, true);
    assert.deepEqual(event.classNames, ['entry-completed']);
    assert.equal(calendarEvent(entry(), []).textColor, '#ffffff');
  } },
];
