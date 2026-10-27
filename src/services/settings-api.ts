import { invoke } from '@tauri-apps/api/core';
import { preferencesSchema } from '@/features/settings/schema';
import type { AppPreferences, BackupPreview, RuntimeStatus } from '@/types/settings';
import { flushSettingsWrites } from './settings-repository';

export async function getPreferences(): Promise<AppPreferences> { await flushSettingsWrites(); return preferencesSchema.parse(await invoke('get_settings')); }
export async function savePreferences(preferences: AppPreferences) { await flushSettingsWrites(); await invoke('save_settings', { preferences: preferencesSchema.parse(preferences) }); }
export async function exportBackup() { return invoke<string | null>('export_backup'); }
export async function previewBackup() { return invoke<BackupPreview | null>('preview_backup'); }
export async function importBackup(token: string) { await flushSettingsWrites(); await invoke('import_backup', { token, confirmed: true }); }
export async function testNotification() { await invoke('test_notification'); }
export async function hideToTray() { await invoke('hide_to_tray'); }
export async function runtimeStatus() { return invoke<RuntimeStatus>('runtime_status'); }
