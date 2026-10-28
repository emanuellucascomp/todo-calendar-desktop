import { describe, expect, it } from 'vitest';
import { entryFormSchema, formDefaults, toEntryDraft } from '../src/features/entries/form';
import { entryTypography } from '../src/features/calendar/typography';
import { movedEntryDraft } from '../src/features/calendar/move';
import type { CalendarEntry } from '../src/types/calendar';

describe('Fonte por registro', () => {
  it('salva os estilos e os recupera no editor sem afetar novos registros', () => {
    const values = formDefaults({ allDay: true });
    Object.assign(values, { title: 'Revisar', font_size: 20, bold: true, italic: true, underline: true });
    const draft = toEntryDraft(entryFormSchema.parse(values), 'UTC');
    const entry = { ...draft, id: 'styled', all_day: 1, completed: 0, created_at: '', updated_at: '', series_parent_id: null } as CalendarEntry;
    expect(formDefaults({ entry })).toMatchObject({ font_size: 20, bold: true, italic: true, underline: true });
    expect(formDefaults({})).toMatchObject({ font_size: 12, bold: false, italic: false, underline: false });
    expect(movedEntryDraft(entry, [], new Date(2026, 10, 3), null, true)).toMatchObject({ font_size: 20, bold: true, italic: true, underline: true });
  });
  it('combina sublinhado e conclusao e rejeita tamanhos invalidos', () => {
    expect(entryTypography({ font_size: 20, bold: true, italic: true, underline: true, completed: 1 })).toEqual({ fontSize: '20px', fontWeight: 700, fontStyle: 'italic', textDecoration: 'underline line-through' });
    const values = { ...formDefaults({}), title: 'Revisar' };
    for (const font_size of [9, 25, 12.5, NaN]) expect(entryFormSchema.safeParse({ ...values, font_size }).success).toBe(false);
  });
});
