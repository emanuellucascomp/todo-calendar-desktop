import type { Category } from '@/types/calendar';
import { getDatabase } from './database';

export async function listCategories(): Promise<Category[]> {
  const db = await getDatabase();
  return db.select<Category[]>('SELECT id, name, color, created_at, updated_at FROM categories ORDER BY created_at, name COLLATE NOCASE');
}
