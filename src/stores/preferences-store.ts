import { create } from 'zustand';
import { DEFAULT_PREFERENCES, type AppPreferences } from '@/types/settings';
import { getPreferences } from '@/services/settings-api';

interface State { preferences: AppPreferences; error: string | null; load: () => Promise<void>; apply: (preferences: AppPreferences) => void }
export const usePreferencesStore = create<State>((set) => ({
  preferences: DEFAULT_PREFERENCES, error: null,
  load: async () => { try { set({ preferences: await getPreferences(), error: null }); } catch(error) { set({ error: `Não foi possível carregar as configurações: ${String(error)}` }); } },
  apply: (preferences) => set({ preferences, error: null }),
}));
