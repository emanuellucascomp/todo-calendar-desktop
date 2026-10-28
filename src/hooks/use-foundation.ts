import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listCategories } from '@/services/categories-repository';
import { loadFoundationSettings, saveShowCompleted } from '@/services/settings-repository';
import type { Category, DatabaseInfo, FoundationSettings } from '@/types/calendar';
import { useCalendarStore } from '@/stores/calendar-store';
import { restoreSession } from '@/services/session-repository';

type FoundationState =
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | { status: 'ready'; categories: Category[]; settings: FoundationSettings; databasePath: string };

export function useFoundation() {
  const [state, setState] = useState<FoundationState>({ status: 'loading' });
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const refreshMetadata = useCallback(async () => {
    try {
      const [categories, settings] = await Promise.all([listCategories(), loadFoundationSettings()]);
      setState((current) => current.status === 'ready' ? { ...current, categories, settings } : current);
    } catch (error) { setSaveError(`Não foi possível atualizar as preferências: ${String(error)}`); }
  }, []);

  useEffect(() => {
    let active = true;
    setState({ status: 'loading' });
    void (async () => {
      // O carregamento do banco acontece antes do comando de diagnóstico.
      const [categories, settings] = await Promise.all([listCategories(), loadFoundationSettings()]);
      const info = await invoke<DatabaseInfo>('database_info');
      if (active) {
        useCalendarStore.getState().setView(settings.lastView);
        await restoreSession();
        if (!active) return;
        setState({ status: 'ready', categories, settings, databasePath: info.path });
      }
    })().catch((error: unknown) => {
      console.error('Falha na inicialização do SQLite:', error);
      if (active) setState({ status: 'error', message: error instanceof Error ? error.message : String(error) });
    });
    return () => { active = false; };
  }, [attempt]);

  const toggleCompleted = useCallback(async () => {
    if (state.status !== 'ready' || saving) return;
    const value = !state.settings.showCompleted;
    setSaving(true);
    setSaveError(null);
    try {
      await saveShowCompleted(value);
      setState((current) => current.status === 'ready'
        ? { ...current, settings: { ...current.settings, showCompleted: value } } : current);
    } catch {
      setSaveError('Não foi possível salvar a preferência. Tente novamente.');
    } finally {
      setSaving(false);
    }
  }, [state, saving]);

  return { state, retry: () => setAttempt((value) => value + 1), refreshMetadata, toggleCompleted, saving, saveError };
}
