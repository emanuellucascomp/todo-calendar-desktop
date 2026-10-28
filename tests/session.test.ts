import { beforeEach, describe, expect, it, vi } from 'vitest';
const db = vi.hoisted(() => ({ select: vi.fn(), execute: vi.fn() }));
vi.mock('../src/services/database', () => ({ getDatabase: async () => db }));
import { restoreSession, saveSession, flushSession } from '../src/services/session-repository';
import { useCalendarStore } from '../src/stores/calendar-store';
import { dateKey } from '../src/features/calendar/dates';

beforeEach(() => {
  useCalendarStore.setState(useCalendarStore.getInitialState(), true);
  db.select.mockReset(); db.execute.mockReset(); db.execute.mockResolvedValue(undefined);
});
describe('Restauração da sessão local', () => {
  it('restaura a data civil, filtros, barra lateral e modo minimalista', async () => {
    db.select.mockResolvedValue([{ value: JSON.stringify({ view: 'week-grid', selectedDate: '2027-01-01', sidebarOpen: false, minimalMode: true, visibleKinds: ['event'], hiddenCategoryIds: ['personal'] }) }]);
    await restoreSession();
    const state = useCalendarStore.getState();
    expect(dateKey(state.selectedDate)).toBe('2027-01-01');
    expect(state.view).toBe('week-grid');
    expect(state.sidebarOpen).toBe(false); expect(state.minimalMode).toBe(true);
    expect(state.visibleKinds).toEqual(['event']); expect(state.hiddenCategoryIds).toEqual(['personal']);
    expect(state.editorRequest).toBeNull();
  });
  it('preserva o padrão quando ainda não existe sessão', async () => {
    db.select.mockResolvedValue([]);
    await restoreSession();
    expect(useCalendarStore.getState().sidebarOpen).toBe(true);
  });
  it('serializa mudanças rápidas e permite aguardar a última gravação antes de fechar', async () => {
    useCalendarStore.getState().selectDate(new Date(2026, 11, 31));
    const first = saveSession();
    useCalendarStore.getState().selectDate(new Date(2027, 0, 1));
    const second = saveSession();
    await Promise.all([first, second, flushSession()]);
    const values = db.execute.mock.calls.map((call) => JSON.parse(call[1][1] as string));
    expect(values.map((value) => value.selectedDate)).toEqual(['2026-12-31', '2027-01-01']);
    expect(values[1]).not.toHaveProperty('editorRequest');
  });
  it('informa falha de gravação e permite tentar novamente', async () => {
    db.execute.mockRejectedValueOnce(new Error('Disco indisponível'));
    await expect(saveSession()).rejects.toThrow('Disco indisponível');
    await expect(flushSession()).rejects.toThrow('Disco indisponível');
    await saveSession(); await flushSession();
  });
});
