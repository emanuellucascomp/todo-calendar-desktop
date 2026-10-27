import { describe, it } from 'vitest';
import { preferenceCases } from './preferences.cases';

describe('Preferência da visualização', () => {
  for (const sample of preferenceCases) it(sample.name, sample.run);
});
