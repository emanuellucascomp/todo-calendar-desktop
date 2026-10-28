import { useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { CalendarColors } from '@/components/calendar/calendar-colors';
import { preferencesSchema } from './schema';
import { exportBackup, getPreferences, hideToTray, importBackup, previewBackup, runtimeStatus, savePreferences, testNotification } from '@/services/settings-api';
import { usePreferencesStore } from '@/stores/preferences-store';
import { useCalendarStore } from '@/stores/calendar-store';
import { DEFAULT_PREFERENCES, type AppPreferences, type BackupPreview, type RuntimeStatus } from '@/types/settings';

export function SettingsDialog({ open, onOpenChange, databasePath, onSaved }: { open: boolean; onOpenChange: (open: boolean) => void; databasePath?: string; onSaved: () => Promise<void> }) {
  const form = useForm<AppPreferences>({ resolver: zodResolver(preferencesSchema), defaultValues: DEFAULT_PREFERENCES });
  const [loading,setLoading] = useState(false); const [busy,setBusy] = useState(false);
  const [loaded,setLoaded] = useState(false);
  const [error,setError] = useState<string | null>(null); const [message,setMessage] = useState<string | null>(null);
  const [preview,setPreview] = useState<BackupPreview | null>(null); const [confirmed,setConfirmed] = useState(false);
  const [status,setStatus] = useState<RuntimeStatus | null>(null);
  const {register,formState:{errors}} = form;
  useEffect(() => {
    if (!open) return;
    let active=true;setLoading(true);setLoaded(false);setError(null);setMessage(null);setPreview(null);setConfirmed(false);
    void Promise.all([getPreferences(),runtimeStatus()]).then(([preferences,status]) => {if(active){form.reset(preferences);setStatus(status);setLoading(false);setLoaded(true);}}).catch((error:unknown)=>{if(active){setError(String(error));setLoading(false);}});
    return()=>{active=false;};
  },[open,form]);

  async function action(operation:()=>Promise<void>) {setBusy(true);setError(null);setMessage(null);try{await operation();}catch(error){setError(String(error));}finally{setBusy(false);}}
  const submit = form.handleSubmit((values) => action(async()=>{
    const previous=usePreferencesStore.getState().preferences;
    await savePreferences(values);usePreferencesStore.getState().apply(values);
    if(previous.default_view!==values.default_view)useCalendarStore.getState().setView(values.default_view);
    await onSaved();useCalendarStore.getState().touch();setMessage('Configurações salvas.');
  }));

  return <Dialog open={open} onOpenChange={(open)=>{if(!busy)onOpenChange(open);}}><DialogContent className="settings-dialog">
    <DialogTitle className="text-xl font-medium">Configurações</DialogTitle><DialogDescription className="mt-1 text-sm text-muted-foreground">Seu calendário, do seu jeito. Todos os dados ficam neste computador.</DialogDescription>
    {error&&<p role="alert" className="form-error">{error}</p>}{message&&<p role="status" className="settings-message">{message}</p>}
    {loading&&<p role="status" className="mt-4 text-sm">Carregando configurações…</p>}
    <form className="entry-form" onSubmit={(event)=>{void submit(event);}} noValidate><fieldset disabled={loading||busy||!loaded||!databasePath}>
      <section className="form-section"><h3>Aparência</h3><label className="form-field">Tema<select {...register('theme')}><option value="light">Claro</option><option value="dark">Escuro</option><option value="system">Seguir tema do sistema</option></select></label>
        <CalendarColors text={form.watch('calendar_text_color')} background={form.watch('calendar_background_color')} onChange={(field, value) => form.setValue(field, value, { shouldDirty: true, shouldValidate: true })} />
        {(errors.calendar_text_color || errors.calendar_background_color) && <p role="alert" className="field-error">Selecione uma cor válida.</p>}
      </section>
      <section className="form-section"><h3>Calendário</h3><div className="form-grid">
        <label className="form-field">Primeiro dia da semana<select {...register('first_day',{valueAsNumber:true})}><option value={0}>Domingo</option><option value={1}>Segunda-feira</option></select></label>
        <label className="form-field">Visualização padrão<select {...register('default_view')}><option value="month">Mês</option><option value="week">Semana com horários</option><option value="week-grid">Semana em grade</option><option value="year">Ano</option></select></label>
      </div><label className="form-check"><input type="checkbox" {...register('show_completed')} /> Mostrar tarefas concluídas</label><p className="field-help">O aplicativo lembra a última visualização. Alterar o padrão também seleciona essa visualização agora.</p></section>
      <section className="form-section"><h3>Notificações</h3><label className="form-check"><input type="checkbox" {...register('notifications_enabled')} /> Ativar lembretes</label>
        <label className="form-field">Antecedência padrão (minutos)<input type="number" min={0} max={525_600} {...register('default_reminder_minutes',{valueAsNumber:true})}/>{errors.default_reminder_minutes&&<span className="field-error" role="alert">{errors.default_reminder_minutes.message}</span>}</label>
        <Button variant="outline" onClick={()=>{void action(async()=>{await testNotification();setMessage('Notificação solicitada ao Windows. Verifique a Central de Notificações.');});}}>Testar notificação</Button>
        <p className="field-help">Lembretes funcionam enquanto o aplicativo estiver aberto ou na bandeja. Com o processo totalmente encerrado, não são emitidos. O Windows pode exigir o aplicativo instalado para exibir notificações.</p>
        {status&&<p className="field-help">{status.last_error?`Último erro do agendador: ${status.last_error}`:status.last_check?`Agendador ativo. Última verificação: ${new Date(status.last_check).toLocaleString('pt-BR',{hour12:false})}`:'Agendador iniciando…'}</p>}
      </section>
      <section className="form-section"><h3>Sistema</h3><label className="form-check"><input type="checkbox" {...register('autostart')} /> Iniciar Manus Calendar com o Windows</label><p className="field-help">Ao entrar na sua conta do Windows, abre o calendário com a data, visualização, filtros e janela da última sessão. Desativado por padrão.</p>
        <label className="form-check"><input type="checkbox" {...register('start_in_tray')} /> Iniciar minimizado na bandeja ao entrar no Windows</label>
        <p className="field-help">Deixe esta opção desmarcada para o calendário aparecer aberto. A sessão é restaurada também ao abrir o aplicativo manualmente.</p>
        <label className="form-check"><input type="checkbox" {...register('minimize_to_tray')} /> Minimizar para a bandeja</label>
        <label className="form-field">Ao fechar a janela<select {...register('close_behavior')}><option value="exit">Encerrar completamente</option><option value="tray">Manter ativo na bandeja</option></select></label>
        <Button variant="outline" onClick={()=>{void action(async()=>{await hideToTray();});}}>Minimizar para a bandeja agora</Button>
      </section>
      <section className="form-section"><h3>Dados</h3><p className="field-help">Localização do banco de dados:</p><code className="database-path">{databasePath??'Banco indisponível'}</code>
        <div className="entry-actions"><Button variant="outline" onClick={()=>{void action(async()=>{const path=await exportBackup();if(path)setMessage(`Backup exportado: ${path}`);});}}>Exportar backup</Button>
          <Button variant="outline" onClick={()=>{void action(async()=>{setPreview(await previewBackup());setConfirmed(false);});}}>Importar backup</Button></div>
        <p className="field-help">O arquivo é validado antes da importação. A restauração substitui os dados em transação e preserva a configuração atual de inicialização com o Windows.</p>
        {preview&&<div className="delete-confirmation"><h4 className="font-medium text-sm">Confirmar restauração</h4><p className="field-help">Backup de {new Date(preview.exported_at).toLocaleString('pt-BR',{hour12:false})}: {preview.entries} registros, {preview.categories} categorias e {preview.exceptions} exceções.</p>
          <label className="form-check"><input type="checkbox" checked={confirmed} onChange={(event)=>setConfirmed(event.target.checked)}/> Entendo que meus dados atuais serão substituídos.</label>
          <div className="entry-actions"><Button variant="outline" onClick={()=>setPreview(null)}>Cancelar</Button><Button className="danger-button" disabled={!confirmed} onClick={()=>{void action(async()=>{await importBackup(preview.token);window.location.reload();});}}>Restaurar backup</Button></div>
        </div>}
      </section>
      <div className="form-actions"><Button variant="outline" onClick={()=>onOpenChange(false)}>Fechar</Button><Button type="submit">{busy?'Salvando…':'Salvar configurações'}</Button></div>
    </fieldset></form>
  </DialogContent></Dialog>;
}
