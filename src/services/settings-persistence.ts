import { z } from 'zod';
import type { CalendarView, FoundationSettings } from '../types/calendar';

export interface SettingsConnection {
  select<T>(query: string, values?: unknown[]): Promise<T>;
  execute(query: string, values?: unknown[]): Promise<unknown>;
}

const firstDaySchema = z.union([z.literal(0), z.literal(1)]);
const viewSchema = z.enum(['month', 'week', 'year']);
const UPSERT = "INSERT INTO app_settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

// Conexão injetável: produção usa o plugin Tauri; testes exercitam o mesmo repositório.
export function createSettingsRepository(connect: () => Promise<SettingsConnection>) {
  let viewWriteQueue: Promise<unknown> = Promise.resolve();

  async function loadFoundationSettings(): Promise<FoundationSettings> {
    const db = await connect();
    const rows = await db.select<{ key: string; value: string }[]>(
      'SELECT key, value FROM app_settings WHERE key IN ($1, $2, $3, $4)',
      ['first_day', 'show_completed', 'last_view', 'default_view'],
    );
    const values = new Map(rows.map((row) => [row.key, row.value]));
    return {
      firstDay: firstDaySchema.parse(JSON.parse(values.get('first_day') ?? '0')),
      showCompleted: z.boolean().parse(JSON.parse(values.get('show_completed') ?? 'true')),
      lastView: viewSchema.parse(JSON.parse(values.get('last_view') ?? values.get('default_view') ?? '"month"')),
    };
  }

  function saveLastView(view: CalendarView): Promise<void> {
    const validated = viewSchema.parse(view);
    // Serializar mudanças rápidas preserva a escolha mais recente.
    const write = viewWriteQueue.catch(() => undefined).then(async () => {
      const db = await connect();
      await db.execute(UPSERT, ['last_view', JSON.stringify(validated)]);
    });
    viewWriteQueue = write;
    return write;
  }

  async function saveShowCompleted(value: boolean): Promise<void> {
    const write = viewWriteQueue.catch(() => undefined).then(async () => {
      const db = await connect(); await db.execute(UPSERT, ['show_completed', JSON.stringify(value)]);
    });
    viewWriteQueue = write; return write;
  }

  return { loadFoundationSettings, saveLastView, saveShowCompleted, flushSettingsWrites: () => viewWriteQueue.catch(() => undefined) };
}
