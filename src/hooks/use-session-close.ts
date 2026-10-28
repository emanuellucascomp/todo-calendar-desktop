import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { saveSession, flushSession } from '@/services/session-repository';
import { flushSettingsWrites } from '@/services/settings-repository';

export function useSessionClose(ready: boolean) {
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!ready) return;
    let active = true;
    let closing = false;
    let stop: (() => void) | undefined;
    void listen<boolean>('session-close-requested', (event) => {
      if (!active || closing) return;
      closing = true;
      void (async () => {
        await saveSession();
        await flushSession();
        await flushSettingsWrites();
        await invoke('finish_session_close', { quit: event.payload });
      })().catch((failure: unknown) => setError(`Não foi possível salvar a sessão antes de fechar: ${String(failure)}`))
        .finally(() => { closing = false; });
    }).then(async (unlisten) => {
      if (!active) { unlisten(); return; }
      stop = unlisten;
      await invoke('session_ready');
    }).catch((failure: unknown) => { if (active) setError(String(failure)); });
    return () => { active = false; stop?.(); };
  }, [ready]);
  return error;
}
