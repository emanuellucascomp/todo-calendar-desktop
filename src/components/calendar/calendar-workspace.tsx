import { useState, type CSSProperties } from 'react';
import { RefreshCw } from 'lucide-react';
import type { Category } from '@/types/calendar';
import { Button } from '@/components/ui/button';
import { useCalendarStore } from '@/stores/calendar-store';
import { useCalendarData } from '@/hooks/use-calendar-data';
import { ScheduleCalendar } from './schedule-calendar';
import { YearCalendar } from './year-calendar';
import { SelectionDialog, type CalendarSelection } from './selection-dialog';
import { completeEntry, moveEntry } from '@/services/entry-actions';
import type { CalendarEntry } from '@/types/calendar';
import { usePreferencesStore } from '@/stores/preferences-store';

export function CalendarWorkspace({ ready, firstDay, showCompleted, categories }: {
  ready: boolean; firstDay: 0 | 1; showCompleted: boolean; categories: Category[];
}) {
  const view = useCalendarStore((state) => state.view);
  const { calendar_text_color: text, calendar_background_color: background } = usePreferencesStore((state) => state.preferences);
  const colors = {
    ...(text ? { '--foreground': text, '--muted-foreground': text } : {}),
    ...(background ? { '--surface': background, '--calendar-custom-background': background } : {}),
  } as CSSProperties;
  const query = useCalendarData(ready, firstDay, showCompleted);
  const [selection, setSelection] = useState<CalendarSelection | null>(null);
  const [saving, setSaving] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  async function toggle(entry: CalendarEntry) {
    if (saving) return;
    setSaving(true); setActionError(null);
    try { await completeEntry(entry); } catch (error) { setActionError(String(error)); } finally { setSaving(false); }
  }

  async function move(entry: CalendarEntry, start: Date, end: Date | null, allDay: boolean) {
    if (saving || !ready) throw new Error('Aguarde o calendário ficar disponível.');
    setSaving(true); setActionError(null);
    try { await moveEntry(entry, start, end, allDay); }
    catch (error) { setActionError(`Não foi possível mover o registro: ${String(error)}`); throw error; }
    finally { setSaving(false); }
  }

  return <>
    {actionError && <p role="alert" className="database-alert">{actionError}</p>}
    {query.error && <div role="alert" className="database-alert">{query.error}<Button variant="outline" size="sm" onClick={query.retry}><RefreshCw /> Tentar novamente</Button></div>}
    <div className="calendar-caption">
      <span>Um lugar para organizar seus dias</span>
      <span role="status">{query.loading ? 'Carregando registros…' : view === 'year' ? 'Visão anual' : 'Clique em uma data ou selecione um período'}</span>
    </div>
    <div className={`calendar-surface${background ? ' custom-calendar-background' : ''}`} style={colors} aria-busy={query.loading}>
      {view === 'year' ? <YearCalendar entries={query.entries} firstDay={firstDay} />
        : <ScheduleCalendar firstDay={firstDay} entries={query.entries} categories={categories} onSelect={setSelection} saving={saving || !ready || query.loading} onMove={move} onToggle={(entry) => { void toggle(entry); }} />}
    </div>
    <SelectionDialog selection={selection} onClose={() => setSelection(null)} entries={query.entries} categories={categories}
      loading={query.loading} error={!ready ? 'Conecte o banco local para consultar os registros.' : query.error} />
  </>;
}
