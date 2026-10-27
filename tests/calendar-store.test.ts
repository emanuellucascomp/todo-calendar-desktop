import { beforeEach, describe, expect, it } from 'vitest';
import { useCalendarStore } from '../src/stores/calendar-store';
import { dateKey } from '../src/features/calendar/dates';

beforeEach(() => useCalendarStore.setState(useCalendarStore.getInitialState(), true));

describe('Navegação e seleção', () => {
  it('abre a semana a partir do ano mantendo a data clicada', () => {
    useCalendarStore.getState().setView('year');
    useCalendarStore.getState().openWeek(new Date(2027, 0, 4));
    expect(useCalendarStore.getState().view).toBe('week');
    expect(dateKey(useCalendarStore.getState().selectedDate)).toBe('2027-01-04');
  });
  it('navega conforme a visualização e Hoje mantém a visualização', () => {
    useCalendarStore.getState().selectDate(new Date(2026, 11, 29));
    useCalendarStore.getState().setView('week');
    useCalendarStore.getState().navigate(1);
    expect(dateKey(useCalendarStore.getState().selectedDate)).toBe('2027-01-05');
    useCalendarStore.getState().today();
    expect(dateKey(useCalendarStore.getState().selectedDate)).toBe(dateKey(new Date()));
    expect(useCalendarStore.getState().view).toBe('week');
  });
  it('trocar a visão preserva a data e os filtros', () => {
    useCalendarStore.getState().selectDate(new Date(2024, 1, 29));
    useCalendarStore.getState().toggleKind('event');
    useCalendarStore.getState().toggleCategory('personal');
    useCalendarStore.getState().setView('year');
    expect(dateKey(useCalendarStore.getState().selectedDate)).toBe('2024-02-29');
    expect(useCalendarStore.getState().visibleKinds).toEqual(['task']);
    expect(useCalendarStore.getState().hiddenCategoryIds).toEqual(['personal']);
  });
});
