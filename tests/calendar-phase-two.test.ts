import { describe, it } from 'vitest';
import { calendarCases } from './calendar-logic.cases';

describe('Calendário — Fase 2', () => {
  for (const sample of calendarCases) it(sample.name, sample.run);
});
