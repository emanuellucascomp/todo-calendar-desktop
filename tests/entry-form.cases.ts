import assert from 'node:assert/strict';
import { entryFormSchema, formDefaults, recurrenceRule, toEntryDraft } from '../src/features/entries/form.ts';

export const formCases: { name: string; run: () => void }[] = [
  {name:'editar uma tarefa sem horário preserva a ausência de horário',run(){
    const values=formDefaults({entry:{id:'untimed',kind:'task',title:'Revisar',description:'',start_date:'2026-11-02',start_time:null,end_date:null,end_time:null,all_day:0,timezone:'UTC',category_id:null,priority:'medium',completed:0,recurrence_rule:null,series_parent_id:null,created_at:'',updated_at:''}});
    assert.equal(values.startTime,'');assert.equal(toEntryDraft(values,'UTC').start_time,null);
  }},
  {name:'formulário mostra a data final UTC da regra no fuso original do registro',run(){
    const values=formDefaults({entry:{id:'until',kind:'task',title:'Estudar',description:'',start_date:'2026-11-02',start_time:'09:00',end_date:null,end_time:null,all_day:0,timezone:'America/Sao_Paulo',category_id:null,priority:'medium',completed:0,recurrence_rule:'FREQ=DAILY;UNTIL=20261105T025959Z',series_parent_id:null,created_at:'',updated_at:''}});
    assert.equal(values.untilDate,'2026-11-04');
  }},
  {name:'campos numéricos ocultos não impedem salvar um registro sem repetição',run(){
    const values=formDefaults({allDay:true});values.title='Sem repetição';values.count=Number.NaN;values.interval=Number.NaN;
    assert.equal(entryFormSchema.safeParse(values).success,true);values.recurrence='custom';assert.equal(entryFormSchema.safeParse(values).success,false);
  }},
  {name:'edição preserva instantes de uma ocorrência durante a repetição do relógio',run(){
    const values=formDefaults({entry:{id:'fold',kind:'event',title:'Evento',description:'',start_date:'2026-11-01',start_time:'01:45',end_date:'2026-11-01',end_time:'01:15',all_day:0,timezone:'America/New_York',category_id:null,priority:'medium',completed:0,recurrence_rule:'FREQ=DAILY;COUNT=2',series_parent_id:null,created_at:'',updated_at:'',start_utc:'2026-11-01T05:45:00Z',end_utc:'2026-11-01T06:15:00Z'}});
    values.title='Título alterado';assert.equal(entryFormSchema.safeParse(values).success,true);const draft=toEntryDraft(values,'America/New_York',true);assert.equal(draft.duration_seconds,1800);assert.equal(draft.end_utc,'2026-11-01T06:15:00Z');
    values.startTime='01:50';assert.equal(entryFormSchema.safeParse(values).success,false);
  }},
  {name:'formulário rejeita dois lembretes com a mesma antecedência',run(){
    const values=formDefaults({allDay:true});values.title='Revisar';values.reminders=[{preset:'10',lead:10,absolute:''},{preset:'custom',lead:10,absolute:''}];
    assert.equal(entryFormSchema.safeParse(values).success,false);
  }},
  {name:'validação rejeita título vazio e término anterior ao início',run(){
    const values=formDefaults({start:new Date(2026,10,2),allDay:true});
    assert.equal(entryFormSchema.safeParse(values).success,false);
    values.title='Reunião';values.kind='event';values.endDate='2026-11-01';
    assert.equal(entryFormSchema.safeParse(values).success,false);
  }},
  {name:'evento de dia inteiro converte término inclusivo do formulário para exclusivo',run(){
    const values=formDefaults({start:new Date(2026,11,31),allDay:true});values.title='Viagem';values.kind='event';values.endDate='2027-01-02';
    const draft=toEntryDraft(entryFormSchema.parse(values),'America/Sao_Paulo');assert.equal(draft.end_date,'2027-01-03');assert.equal(draft.start_time,null);
  }},
  {name:'evento com horário exige duração positiva e tarefa pode não ter horário',run(){
    const values=formDefaults({start:new Date(2026,10,2),allDay:false});values.title='Tarefa';values.startTime='';
    assert.equal(entryFormSchema.safeParse(values).success,true);
    values.kind='event';values.startTime='10:00';values.endDate=values.startDate;values.endTime='09:00';assert.equal(entryFormSchema.safeParse(values).success,false);
    values.endTime='11:00';assert.equal(entryFormSchema.safeParse(values).success,true);
  }},
  {name:'formulário gera RRULE personalizada com intervalo, dias e limite',run(){
    const values=formDefaults({allDay:true});values.title='Estudar';values.recurrence='custom';values.frequency='WEEKLY';values.interval=2;values.weekdays=['MO','WE'];values.ending='count';values.count=6;
    assert.equal(recurrenceRule(values),'FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=6');
  }},
  {name:'lembretes absolutos são validados e limitados a registros ou ocorrências individuais',run(){
    const values=formDefaults({allDay:true});values.title='Revisar';values.recurrence='daily';values.reminders=[{preset:'absolute',lead:10,absolute:'2026-11-02T09:00'}];
    assert.equal(entryFormSchema.safeParse(values).success,false);values.scope='one';assert.equal(entryFormSchema.safeParse(values).success,true);
    const draft=toEntryDraft(values,'America/Sao_Paulo',true);assert.equal(draft.recurrence_rule,null);assert.equal(draft.reminders[0]?.kind,'absolute');
  }},
];
