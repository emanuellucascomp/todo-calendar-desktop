// Verificação adicional sem npm: Node 22.18+ remove os tipos nativamente.
// Não substitui Vitest, typecheck ou testes do runtime Tauri.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dateKey, monthCells, shiftMonth } from '../src/features/calendar/dates.ts';
import { calendarCases } from './calendar-logic.cases.ts';
import { preferenceCases } from './preferences.cases.ts';
import { useCalendarStore } from '../src/stores/calendar-store.ts';
import { formCases } from './entry-form.cases.ts';
import { settingsCases } from './settings.cases.ts';

test('navegação preserva dia válido em fevereiro e anos bissextos', () => {
  assert.equal(dateKey(shiftMonth(new Date(2026, 0, 31), 1)), '2026-02-28');
  assert.equal(dateKey(shiftMonth(new Date(2024, 0, 31), 1)), '2024-02-29');
  assert.equal(dateKey(shiftMonth(new Date(2024, 2, 31), -1)), '2024-02-29');
});
test('navegação atravessa limites de ano', () => {
  assert.equal(dateKey(shiftMonth(new Date(2026, 11, 31), 1)), '2027-01-31');
  assert.equal(dateKey(shiftMonth(new Date(2026, 0, 31), -1)), '2025-12-31');
});
test('mini calendário começa no dia escolhido e cobre cada mês', () => {
  for (let year = 2024; year <= 2028; year++) {
    for (let month = 0; month < 12; month++) {
      for (const firstDay of [0, 1] as const) {
        const cells = monthCells(new Date(year, month, 1), firstDay);
        assert.equal(cells.length, 42);
        assert.equal(cells[0]?.getDay(), firstDay);
        assert.equal(new Set(cells.map(dateKey)).size, 42);
        const count = cells.filter((date) => date.getMonth() === month && date.getFullYear() === year).length;
        assert.equal(count, new Date(year, month + 1, 0).getDate());
      }
    }
  }
});
test('data civil não muda para UTC implicitamente', () => {
  assert.equal(dateKey(new Date(2026, 10, 2, 0, 30)), '2026-11-02');
});

for (const sample of calendarCases) test(sample.name, sample.run);
for (const sample of preferenceCases) test(sample.name, sample.run);
for (const sample of formCases) test(sample.name, sample.run);
for (const sample of settingsCases) test(sample.name, sample.run);

test('store abre a semana do dia escolhido e mantém visão ao usar Hoje', () => {
  useCalendarStore.setState(useCalendarStore.getInitialState(), true);
  useCalendarStore.getState().setView('year');
  useCalendarStore.getState().openWeek(new Date(2027, 0, 4));
  assert.equal(useCalendarStore.getState().view, 'week');
  assert.equal(dateKey(useCalendarStore.getState().selectedDate), '2027-01-04');
  useCalendarStore.getState().navigate(-1);
  assert.equal(dateKey(useCalendarStore.getState().selectedDate), '2026-12-28');
  useCalendarStore.getState().today();
  assert.equal(dateKey(useCalendarStore.getState().selectedDate), dateKey(new Date()));
  assert.equal(useCalendarStore.getState().view, 'week');
});
