import { describe, expect, it } from 'vitest';
import { dateKey, monthCells, shiftMonth } from '../src/features/calendar/dates';

describe('Datas civis do calendário', () => {
  it('limita o dia ao navegar de janeiro para fevereiro', () => {
    expect(dateKey(shiftMonth(new Date(2026, 0, 31, 12), 1))).toBe('2026-02-28');
    expect(dateKey(shiftMonth(new Date(2024, 0, 31, 12), 1))).toBe('2024-02-29');
  });
  it('navega em ambas as direções na mudança de ano', () => {
    expect(dateKey(shiftMonth(new Date(2026, 11, 31, 12), 1))).toBe('2027-01-31');
    expect(dateKey(shiftMonth(new Date(2026, 0, 1, 12), -1))).toBe('2025-12-01');
  });
  it.each([0, 1] as const)('gera 42 datas únicas com primeiro dia %i', (firstDay) => {
    const days = monthCells(new Date(2026, 9, 26, 12), firstDay);
    expect(days).toHaveLength(42);
    expect(days[0]?.getDay()).toBe(firstDay);
    expect(new Set(days.map(dateKey)).size).toBe(42);
  });
  it('mantém a data civil no fuso local', () => {
    expect(dateKey(new Date(2026, 10, 2, 0, 30))).toBe('2026-11-02');
  });
});
