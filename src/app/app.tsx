import { useEffect, useState } from 'react';
import { AlertCircle, Database, Minimize2, Maximize2, RefreshCw } from 'lucide-react';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';
import { CalendarWorkspace } from '@/components/calendar/calendar-workspace';
import { SettingsDialog } from '@/features/settings/settings-dialog';
import { Button } from '@/components/ui/button';
import { useFoundation } from '@/hooks/use-foundation';
import { useCalendarStore } from '@/stores/calendar-store';
import { useCalendarPreferences } from '@/hooks/use-calendar-preferences';
import { EntryEditor } from '@/components/entries/entry-editor';
import { CategoryManager } from '@/components/entries/category-manager';
import { usePreferencesStore } from '@/stores/preferences-store';
import { useAppearance } from '@/hooks/use-appearance';

export function App() {
  const { state, retry, refreshMetadata, toggleCompleted, saving, saveError } = useFoundation();
  const sidebarOpen = useCalendarStore((store) => store.sidebarOpen);
  const minimalMode = useCalendarStore((store) => store.minimalMode);
  const toggleMinimalMode = useCalendarStore((store) => store.toggleMinimalMode);
  const editorRequest = useCalendarStore((store) => store.editorRequest);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const ready = state.status === 'ready';
  const preferences = useCalendarPreferences(ready);
  const appPreferences = usePreferencesStore((store) => store.preferences);
  const preferencesError = usePreferencesStore((store) => store.error);
  useAppearance();
  useEffect(() => { if (ready) void usePreferencesStore.getState().load(); }, [ready]);
  return (
    <div className={`app-shell${minimalMode ? ' minimal-mode' : ''}`}>
      <a className="skip-link" href="#calendar">Ir para o calendário</a>
      <Button variant="outline" size="icon" className="minimal-toggle" aria-label={minimalMode ? 'Mostrar interface completa' : 'Ocultar cabeçalho e focar no calendário'} title={minimalMode ? 'Mostrar interface completa' : 'Modo minimalista'} aria-pressed={minimalMode} onClick={toggleMinimalMode}>
        {minimalMode ? <Minimize2 /> : <Maximize2 />}
      </Button>
      {!minimalMode && <Header onSettings={() => setSettingsOpen(true)} firstDay={ready ? state.settings.firstDay : 0} loading={state.status === 'loading'} />}
      <div className="app-body">
        {sidebarOpen && !minimalMode && <Sidebar categories={ready ? state.categories : []} firstDay={ready ? state.settings.firstDay : 0}
          ready={ready} showCompleted={ready ? state.settings.showCompleted : true} saving={saving} toggleCompleted={() => { void toggleCompleted(); }} />}
        <main id="calendar" tabIndex={-1} className="calendar-main" aria-label="Calendário de tarefas e eventos">
          {state.status === 'error' && <div role="alert" className="database-alert">
            <AlertCircle className="size-5 shrink-0" /><div><strong>Não foi possível conectar o banco local.</strong><p>{state.message}</p></div>
            <Button variant="outline" size="sm" className="ml-auto shrink-0" onClick={retry}><RefreshCw /> Tentar novamente</Button>
          </div>}
          {saveError && <p role="alert" className="database-alert">{saveError}</p>}
          {preferencesError && <p role="alert" className="database-alert">{preferencesError}</p>}
          {preferences.error && <div role="alert" className="database-alert">{preferences.error}
            <Button variant="outline" size="sm" onClick={preferences.retry}><RefreshCw /> Tentar salvar</Button>
          </div>}
          <CalendarWorkspace ready={ready} firstDay={ready ? state.settings.firstDay : 0}
            showCompleted={ready ? state.settings.showCompleted : true} categories={ready ? state.categories : []} />
          <footer className="calendar-footer" aria-live="polite">
            <span className="flex items-center gap-2"><Database className="size-3.5" />{ready ? 'SQLite conectado · dados neste computador' : state.status === 'loading' ? 'Conectando ao banco local…' : 'Armazenamento indisponível'}</span>
            <span>Português (Brasil) · 24 horas</span>
          </footer>
        </main>
      </div>
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} databasePath={ready ? state.databasePath : undefined} onSaved={refreshMetadata} />
      {ready && editorRequest && <EntryEditor request={editorRequest} categories={state.categories} defaultReminder={appPreferences.default_reminder_minutes} />}
      {ready && <CategoryManager categories={state.categories} onChanged={() => { void refreshMetadata(); }} />}
    </div>
  );
}
