import { useEffect, useState } from 'react';
import { useFieldArray, useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Plus, Trash2 } from 'lucide-react';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { entryFormSchema, formDefaults, recurrenceRule, toEntryDraft, type EntryFormValues } from '@/features/entries/form';
import { getEntryDetails, saveEntry } from '@/services/entry-actions';
import { useCalendarStore } from '@/stores/calendar-store';
import type { Category, EditorRequest } from '@/types/calendar';

export function EntryEditor({ request, categories, defaultReminder }: { request: EditorRequest; categories: Category[]; defaultReminder: number }) {
  const close = useCalendarStore((state) => state.closeEditor);
  const [loading, setLoading] = useState(Boolean(request.entry));
  const [loadFailed, setLoadFailed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const form = useForm<EntryFormValues>({ resolver: zodResolver(entryFormSchema), defaultValues: formDefaults(request) });
  const { register, watch, setValue, formState: { errors, isSubmitting } } = form;
  const reminders = useFieldArray({ control: form.control, name: 'reminders' });
  const values = watch();
  const individual = Boolean(request.entry?.recurrence_rule) && values.scope === 'one';

  useEffect(() => {
    if (!request.entry) return;
    let active = true;
    void getEntryDetails(request.entry).then((details) => {
      if (active) { form.reset(formDefaults({ entry: details.occurrence }, details.reminders)); setLoading(false); }
    }).catch((error: unknown) => { if (active) { setError(String(error)); setLoadFailed(true); setLoading(false); } });
    return () => { active = false; };
  }, [request, form]);

  const submit = form.handleSubmit(async (values) => {
    setError(null);
    try {
      const timezone = request.entry?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone;
      const draft = toEntryDraft(values, timezone, individual);
      // Preserve inclusive UNTIL timestamps and canonical RRULEs when repetition was not edited.
      if (!individual && request.entry?.recurrence_rule && recurrenceRule(values) === recurrenceRule(formDefaults(request))) draft.recurrence_rule = request.entry.recurrence_rule;
      await saveEntry(request.entry, draft, values.scope);
      close();
    } catch (error) { setError(String(error)); }
  });

  return <Dialog open onOpenChange={(open) => { if (!open && !isSubmitting) close(); }}><DialogContent className="entry-editor">
    <DialogTitle className="text-xl font-medium">{request.entry ? 'Editar registro' : 'Criar registro'}</DialogTitle>
    <DialogDescription className="mt-1 text-sm text-muted-foreground">Organize uma tarefa ou um evento no seu calendário.</DialogDescription>
    <form onSubmit={(event) => { void submit(event); }} className="entry-form" noValidate>
      {error && <p role="alert" className="form-error">{error}</p>}
      {loading && <p role="status">Carregando detalhes…</p>}
      <fieldset disabled={loading || isSubmitting || loadFailed}>
        {request.entry?.recurrence_rule && <label className="form-field">Aplicar alteração a
          <select {...register('scope')}><option value="one">Somente esta ocorrência</option><option value="future">Esta e as próximas ocorrências</option><option value="series">Toda a série</option></select>
        </label>}
        {request.entry?.recurrence_rule && values.scope === 'series' && <p className="field-help">A série será atualizada preservando as ocorrências com ajustes individuais e suas conclusões. Uma regra nova pode deixar algumas datas antigas fora da série.</p>}
        <div className="form-grid">
          <label className="form-field">Tipo<select {...register('kind')} onChange={(event) => {
            setValue('kind', event.target.value as EntryFormValues['kind']);
            if (event.target.value === 'event' && !values.endDate) setValue('endDate', values.startDate);
            if (event.target.value === 'event' && !values.allDay) { if (!values.startTime) setValue('startTime','09:00'); if (!values.endTime) setValue('endTime','10:00'); }
          }}><option value="task">Tarefa</option><option value="event">Evento</option></select></label>
          <label className="form-field">Prioridade<select {...register('priority')} disabled={values.kind === 'event'}><option value="low">Baixa</option><option value="medium">Média</option><option value="high">Alta</option></select></label>
        </div>
        <label className="form-field">Título<input autoFocus maxLength={300} {...register('title')} placeholder="Adicionar título" aria-invalid={!!errors.title} /><FieldError message={errors.title?.message} /></label>
        <label className="form-field">Descrição<textarea rows={3} {...register('description')} placeholder="Adicionar uma descrição" /><FieldError message={errors.description?.message} /></label>
        <label className="form-check"><input type="checkbox" {...register('allDay')} onChange={(event) => {
          setValue('allDay',event.target.checked);
          if (!event.target.checked && values.kind === 'event') {setValue('startTime',values.startTime||'09:00');setValue('endTime',values.endTime||'10:00');}
        }} /> Dia inteiro / sem horário</label>
        <div className="form-grid">
          <label className="form-field">Data inicial<input type="date" {...register('startDate')} /><FieldError message={errors.startDate?.message} /></label>
          <label className="form-field">Data final {values.kind === 'task' && '(opcional)'}<input type="date" {...register('endDate')} /><FieldError message={errors.endDate?.message} /></label>
          {!values.allDay && <>
            <label className="form-field">Horário inicial {values.kind === 'task' && '(opcional)'}<input type="time" {...register('startTime')} /><FieldError message={errors.startTime?.message} /></label>
            <label className="form-field">Horário final {values.kind === 'task' && '(opcional)'}<input type="time" {...register('endTime')} /><FieldError message={errors.endTime?.message} /></label>
          </>}
        </div>
        {values.allDay && <p className="field-help">A data final inclui o último dia do registro. Lembretes de registros sem horário usam 09:00 no fuso do registro.</p>}
        <p className="field-help">Fuso dos horários deste registro: {request.entry?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone}.</p>
        {values.referenceStartUtc && values.referenceEndUtc && values.startDate === values.endDate && values.endTime <= values.startTime && <p className="field-help">Este intervalo atravessa uma repetição do relógio. Os instantes e a duração serão preservados enquanto os horários não forem alterados.</p>}
        <label className="form-field">Categoria<select {...register('categoryId')}><option value="">Sem categoria</option>{categories.map((category) => <option value={category.id} key={category.id}>{category.name}</option>)}</select></label>
        {!individual && <section className="form-section"><h3>Repetição</h3>
          <label className="form-field"><span className="sr-only">Repetir</span><select {...register('recurrence')}>
            <option value="none">Não repetir</option><option value="daily">Todos os dias</option><option value="weekdays">Todos os dias úteis</option><option value="weekly">Semanalmente</option><option value="monthly">Mensalmente</option><option value="yearly">Anualmente</option><option value="custom">Personalizada</option>
          </select></label>
          {values.recurrence === 'custom' && <>
            <div className="form-grid"><label className="form-field">Frequência<select {...register('frequency')}><option value="DAILY">Diária</option><option value="WEEKLY">Semanal</option><option value="MONTHLY">Mensal</option><option value="YEARLY">Anual</option></select></label>
              <label className="form-field">A cada<input type="number" min={1} max={999} {...register('interval', { valueAsNumber: true })} /><FieldError message={errors.interval?.message} /></label></div>
            <div className="weekday-options">{(['MO','TU','WE','TH','FR','SA','SU'] as const).map((day,index) => <label key={day}><input type="checkbox" value={day} {...register('weekdays')} />{['Seg','Ter','Qua','Qui','Sex','Sáb','Dom'][index]}</label>)}</div>
            <p className="field-help">Sem dias selecionados, repete a data original. Em meses/anos com dias selecionados, inclui todos esses dias da semana no período.</p>
          </>}
          {values.recurrence !== 'none' && <>
            <label className="form-field">Termina<select {...register('ending')}><option value="never">Nunca</option><option value="until">Em uma data</option><option value="count">Após uma quantidade</option></select></label>
            {values.ending === 'until' && <label className="form-field">Data de encerramento<input type="date" {...register('untilDate')} /><FieldError message={errors.untilDate?.message} /></label>}
            {values.ending === 'count' && <label className="form-field">Quantidade máxima de ocorrências<input type="number" min={1} max={100_000} {...register('count',{valueAsNumber:true})} /><FieldError message={errors.count?.message} /></label>}
            <p className="field-help">Datas inexistentes são puladas: dia 31 não vira dia 28, e 29 de fevereiro ocorre somente em anos bissextos.</p>
          </>}
        </section>}
        <section className="form-section"><div className="form-section-heading"><h3>Lembretes</h3><Button variant="ghost" size="sm" disabled={reminders.fields.length >= 12} onClick={() => reminders.append({preset:'custom',lead:defaultReminder,absolute:''})}><Plus /> Adicionar</Button></div>
          {reminders.fields.length === 0 && <p className="field-help">Nenhum lembrete configurado.</p>}
          {reminders.fields.map((field,index) => <div className="reminder-row" key={field.id}>
            <select aria-label={`Lembrete ${index+1}`} {...register(`reminders.${index}.preset`)}>
              <option value="0">No horário</option><option value="5">5 minutos antes</option><option value="10">10 minutos antes</option><option value="15">15 minutos antes</option><option value="30">30 minutos antes</option><option value="60">1 hora antes</option><option value="1440">1 dia antes</option><option value="custom">Antecedência personalizada</option><option value="absolute">Data e horário personalizados</option>
            </select>
            <FieldError message={errors.reminders?.[index]?.preset?.message} />
            {values.reminders[index]?.preset === 'custom' && <label className="form-field">Minutos antes<input type="number" min={0} max={525_600} {...register(`reminders.${index}.lead`,{valueAsNumber:true})} /><FieldError message={errors.reminders?.[index]?.lead?.message} /></label>}
            {values.reminders[index]?.preset === 'absolute' && <label className="form-field">Data e horário neste computador<input type="datetime-local" {...register(`reminders.${index}.absolute`)} /><FieldError message={errors.reminders?.[index]?.absolute?.message} /></label>}
            <Button variant="ghost" size="icon" aria-label={`Remover lembrete ${index+1}`} onClick={() => reminders.remove(index)}><Trash2 /></Button>
          </div>)}
        </section>
      </fieldset>
      <div className="form-actions"><Button variant="outline" onClick={close} disabled={isSubmitting}>Cancelar</Button><Button type="submit" disabled={isSubmitting || loading || loadFailed}>{isSubmitting ? 'Salvando…' : 'Salvar'}</Button></div>
    </form>
  </DialogContent></Dialog>;
}

function FieldError({ message }: { message?: string }) { return message ? <span role="alert" className="field-error">{message}</span> : null; }
