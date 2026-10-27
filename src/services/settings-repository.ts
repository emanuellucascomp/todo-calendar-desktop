import { getDatabase } from './database';
import { createSettingsRepository } from './settings-persistence';

export const { loadFoundationSettings, saveLastView, saveShowCompleted, flushSettingsWrites } = createSettingsRepository(getDatabase);
