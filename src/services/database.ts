import Database from '@tauri-apps/plugin-sql';
import { isTauri } from '@tauri-apps/api/core';

export const DATABASE_URL = 'sqlite:todo-calendar.db';
let connection: Promise<Database> | undefined;

export function getDatabase(): Promise<Database> {
  if (!isTauri()) {
    return Promise.reject(new Error('O SQLite está disponível no aplicativo desktop. Abra o Manus Calendar com o Tauri.'));
  }
  connection ??= connect().catch((error: unknown) => {
    connection = undefined;
    throw error;
  });
  return connection;
}

async function connect(): Promise<Database> {
  const db = await Database.load(DATABASE_URL);
  // SQLx ativa FK em todas as conexões. Um PRAGMA execute isolado não configuraria o pool inteiro.
  const rows = await db.select<{ foreign_keys: number }[]>('PRAGMA foreign_keys');
  if (rows[0]?.foreign_keys !== 1) {
    await db.close();
    throw new Error('A conexão SQLite não ativou a integridade referencial.');
  }
  return db;
}
