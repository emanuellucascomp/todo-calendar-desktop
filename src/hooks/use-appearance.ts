import { useEffect } from 'react';
import { usePreferencesStore } from '@/stores/preferences-store';
export function useAppearance() {
  const theme = usePreferencesStore((state) => state.preferences.theme);
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const update = () => { document.documentElement.dataset.theme = theme === 'system' ? media.matches ? 'dark' : 'light' : theme; };
    update(); media.addEventListener('change', update); return () => media.removeEventListener('change', update);
  }, [theme]);
}
