import { useEffect, useMemo, useState } from 'react';
import { format, isSameDay } from 'date-fns';
import { ptBR } from 'date-fns/locale';
import { CalendarDays, CalendarRange, Square, SquareCheck } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { entryTypography } from '@/features/calendar/typography';
import { entryInterval } from '@/features/calendar/entries';
import type { CalendarEntry, Category } from '@/types/calendar';
import { useCalendarStore } from '@/stores/calendar-store';
import { duplicateEntry, removeEntry, completeEntry } from '@/services/entry-actions';
import type { EditScope } from '@/types/calendar';
import { timedIntervalLabel } from '@/features/calendar/presentation';

export interface CalendarSelection { start: Date; end: Date; allDay: boolean; entryId?: string }

export function SelectionDialog({ selection, onClose, entries, categories, loading, error }: {
  selection: CalendarSelection | null;
  onClose: () => void;
  entries: CalendarEntry[];
  categories: Category[];
  loading: boolean;
  error: string | null;
}) {
  const openWeek = useCalendarStore((state) => state.openWeek);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [scope, setScope] = useState<EditScope>('one');
  useEffect(() => { setConfirmDelete(null); setActionError(null); setScope('one'); }, [selection]);
  async function mutate(action: () => Promise<void>, close = false) {
    setBusy(true); setActionError(null);
    try { await action(); if (close) onClose(); } catch (error) { setActionError(String(error)); } finally { setBusy(false); }
  }
  const selected = useMemo(() => !selection ? [] : entries.filter((entry) => {
    if (selection.entryId) return entry.id === selection.entryId;
    const interval = entryInterval(entry);
    return interval.start < selection.end && interval.end > selection.start;
  }), [selection, entries]);
  const print = (date: Date, pattern: string) => format(date, pattern, { locale: ptBR });
  const lastDay = selection ? new Date(selection.end.getTime() - 1) : null;

  return <Dialog open={selection !== null} onOpenChange={(open) => { if (!open) onClose(); }}><DialogContent>
    <DialogTitle className="pr-5 text-xl font-medium">{selection?.entryId ? 'Detalhes do registro' : 'Registros do período'}</DialogTitle>
    <DialogDescription className="mt-2 text-sm text-muted-foreground">
      {selection && (selection.allDay
        ? lastDay && isSameDay(selection.start, lastDay)
          ? print(selection.start, "EEEE, d 'de' MMMM 'de' yyyy")
          : `${print(selection.start, 'dd/MM/yyyy')} – ${lastDay ? print(lastDay, 'dd/MM/yyyy') : ''}`
        : timedIntervalLabel(selection.start,selection.end))}
    </DialogDescription>
    {actionError && <p role="alert" className="form-error">{actionError}</p>}
    <div className="context-entries" aria-live="polite">
      {loading ? <p>Carregando registros…</p> : error ? <p role="alert">{error}</p>
        : selected.length === 0 ? <div className="context-empty"><CalendarDays /><p>Nenhum registro neste período.</p></div>
          : selected.map((entry) => {
            const category = categories.find((item) => item.id === entry.category_id);
            const interval = entryInterval(entry);
            return <article key={entry.id} className="context-entry" style={{ borderLeftColor: category?.color ?? '#1A73E8' }}>
              <h3 style={entryTypography(entry)}>
                {entry.kind === 'event' ? <CalendarDays /> : entry.completed ? <SquareCheck /> : <Square />}{entry.title}
              </h3>
              <p>{entry.kind === 'task' ? entry.completed ? 'Tarefa concluída' : 'Tarefa pendente' : 'Evento'}{category ? ` · ${category.name}` : ''}</p>
              {entry.kind === 'task' && <p>Prioridade: {{low:'Baixa',medium:'Média',high:'Alta'}[entry.priority]}</p>}
              <p>{interval.allDay ? `Dia inteiro · ${print(interval.start, 'dd/MM/yyyy')}` : timedIntervalLabel(interval.start,interval.end)}</p>
              {entry.description && <div className="context-description">{entry.description}</div>}
              <div className="entry-actions">
                <Button variant="outline" size="sm" disabled={busy} onClick={() => { onClose(); useCalendarStore.getState().openEditor(entry); }}>Editar</Button>
                <Button variant="ghost" size="sm" disabled={busy} onClick={() => { void mutate(() => duplicateEntry(entry), true); }}>{entry.recurrence_rule ? 'Duplicar ocorrência' : 'Duplicar'}</Button>
                {entry.kind === 'task' && <Button variant="ghost" size="sm" disabled={busy} onClick={() => { void mutate(() => completeEntry(entry)); }}>{entry.completed ? 'Reabrir' : 'Concluir'}</Button>}
                <Button variant="ghost" size="sm" className="danger-text" disabled={busy} onClick={() => setConfirmDelete(entry.id)}>Excluir</Button>
              </div>
              {confirmDelete === entry.id && <div className="delete-confirmation">
                <p>Excluir este registro? Esta ação não pode ser desfeita.</p>
                {entry.recurrence_rule && <label className="form-field">Excluir<select value={scope} onChange={(event) => setScope(event.target.value as EditScope)}><option value="one">Somente esta ocorrência</option><option value="future">Esta e as próximas ocorrências</option><option value="series">Toda a série</option></select></label>}
                <div className="entry-actions"><Button variant="outline" size="sm" disabled={busy} onClick={() => setConfirmDelete(null)}>Cancelar</Button><Button size="sm" className="danger-button" disabled={busy} onClick={() => {void mutate(() => removeEntry(entry, entry.recurrence_rule ? scope : 'series'), true);}}>Confirmar exclusão</Button></div>
              </div>}
            </article>;
          })}
    </div>
    <div className="entry-actions"><Button variant="outline" onClick={() => { if (selection) openWeek(selection.start); onClose(); }}><CalendarRange /> Abrir esta semana</Button>
      {!error && <Button onClick={() => { if (selection) useCalendarStore.getState().openCreate({ start: selection.start, end: selection.end, allDay: selection.allDay }); onClose(); }}>Criar neste período</Button>}
    </div>
  </DialogContent></Dialog>;
}
