import { describe, expect, it } from 'vitest';
import { movedEntryDraft } from '../src/features/calendar/move';
import { entry } from './calendar-logic.cases';

describe('Mover registros', () => {
  it('preserva duração exata durante a mudança de horário de verão e fuso da série', () => {
    const original = entry({ timezone: 'America/New_York', start_date: '2026-11-01', start_time: '01:30', end_date: '2026-11-01', end_time: '01:30', start_utc: '2026-11-01T05:30:00Z', end_utc: '2026-11-01T06:30:00Z', recurrence_rule: 'FREQ=DAILY' });
    const draft = movedEntryDraft(original, [], new Date('2026-11-02T06:30:00Z'), new Date('2026-11-02T06:30:00Z'), false);
    expect(draft.start_time).toBe('01:30'); expect(draft.end_time).toBe('02:30');
    expect(draft.duration_seconds).toBe(3600); expect(draft.timezone).toBe(original.timezone);
    expect(draft.recurrence_rule).toBeNull(); expect(original.start_date).toBe('2026-11-01');
  });
  it('mantém término exclusivo, metadados e lembretes ao mover dia inteiro', () => {
    const reminders = [{ kind: 'relative' as const, minutes_before: 10, scheduled_at: null }];
    const draft = movedEntryDraft(entry({ all_day: 1 }), reminders, new Date(2026, 11, 31), new Date(2027, 0, 2), true);
    expect(draft.start_date).toBe('2026-12-31'); expect(draft.end_date).toBe('2027-01-02');
    expect(draft.start_time).toBeNull(); expect(draft.start_utc).toBeNull(); expect(draft.reminders).toEqual(reminders);
    expect(draft.category_id).toBe('personal');
  });
  it('converte dia inteiro em horário com término padrão quando necessário', () => {
    const draft = movedEntryDraft(entry({ all_day: 1 }), [], new Date('2026-11-02T12:00:00Z'), null, false);
    expect(draft.start_time).toBe('09:00'); expect(draft.end_time).toBe('09:30'); expect(draft.duration_seconds).toBe(1800);
  });
});
