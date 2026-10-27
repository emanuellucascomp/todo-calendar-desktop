import assert from 'node:assert/strict';
import { createSettingsRepository, type SettingsConnection } from '../src/services/settings-persistence.ts';

// O driver de teste é isolado; nunca faz parte do aplicativo.
function driver(rows: { key: string; value: string }[] = []) {
  const writes: unknown[][] = [];
  const connection: SettingsConnection = {
    select: async <T>() => rows as T,
    execute: async (_query, values = []) => { writes.push(values); },
  };
  return { connection, writes, repository: createSettingsRepository(async () => connection) };
}

export const preferenceCases: { name: string; run: () => Promise<void> }[] = [
  { name: 'restaura a última visão e o primeiro dia salvos', async run() {
    const { repository } = driver([{ key: 'last_view', value: '"year"' }, { key: 'first_day', value: '1' }]);
    assert.deepEqual(await repository.loadFoundationSettings(), { firstDay: 1, lastView: 'year', showCompleted: true });
  } },
  { name: 'usa a visão padrão e depois mensal quando não há última visão', async run() {
    assert.equal((await driver([{ key: 'default_view', value: '"week"' }]).repository.loadFoundationSettings()).lastView, 'week');
    assert.equal((await driver().repository.loadFoundationSettings()).lastView, 'month');
  } },
  { name: 'rejeita preferência corrompida', async run() {
    await assert.rejects(driver([{ key: 'last_view', value: '"invalid"' }]).repository.loadFoundationSettings());
  } },
  { name: 'serializa alterações enquanto a primeira gravação está pendente', async run() {
    const { repository, connection, writes } = driver();
    let release: () => void = () => { throw new Error('Gravação não iniciada'); };
    let announce: () => void = () => undefined;
    const started = new Promise<void>((resolve) => { announce = resolve; });
    connection.execute = async (_query, values = []) => {
      writes.push(values);
      if (writes.length === 1) {
        const gate = new Promise<void>((resolve) => { release = resolve; });
        announce();
        await gate;
      }
    };
    const first = repository.saveLastView('week');
    const second = repository.saveLastView('year');
    await started;
    assert.equal(writes.length, 1);
    release();
    await Promise.all([first, second]);
    assert.deepEqual(writes, [['last_view', '"week"'], ['last_view', '"year"']]);
  } },
  { name: 'erro de gravação não bloqueia a próxima preferência', async run() {
    const { repository, connection } = driver();
    let fail = true;
    connection.execute = async () => { if (fail) throw new Error('Falha no disco'); };
    await assert.rejects(repository.saveLastView('week'), /Falha no disco/);
    fail = false;
    await repository.saveLastView('month');
  } },
];
