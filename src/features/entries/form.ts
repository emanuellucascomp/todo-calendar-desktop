import { z } from 'zod';
import { dateKey, parseCivilDate } from '../calendar/dates.ts';
import type { CalendarEntry, EditorRequest, EntryDraft, ReminderDraft } from '../../types/calendar';

const validDate = (value: string) => { try { parseCivilDate(value); return true; } catch { return false; } };
const weekdays = ['MO', 'TU', 'WE', 'TH', 'FR', 'SA', 'SU'] as const;
const presets = ['none', 'daily', 'weekdays', 'weekly', 'monthly', 'yearly', 'custom'] as const;
interface ScheduleValues { startDate:string; startTime:string; endDate:string; endTime:string; allDay:boolean }
function scheduleKey(values:ScheduleValues) {return JSON.stringify([values.startDate,values.startTime,values.endDate,values.endTime,values.allDay]);}
function preservedInterval(values:ScheduleValues & {referenceStartUtc:string;referenceEndUtc:string;referenceScheduleKey:string}) {
  return !values.allDay&&scheduleKey(values)===values.referenceScheduleKey&&Number.isFinite(Date.parse(values.referenceStartUtc))&&Date.parse(values.referenceEndUtc)>Date.parse(values.referenceStartUtc);
}
function untilDate(rule:string|undefined,timezone:string):string {
  if (!rule) return '';
  if (rule.endsWith('Z')) {
    const instant=new Date(`${rule.slice(0,4)}-${rule.slice(4,6)}-${rule.slice(6,8)}T${rule.slice(9,11)}:${rule.slice(11,13)}:${rule.slice(13,15)}Z`);
    const parts=new Intl.DateTimeFormat('en-CA',{timeZone:timezone,year:'numeric',month:'2-digit',day:'2-digit'}).formatToParts(instant);
    const value=(type:string)=>parts.find(part=>part.type===type)?.value??'';
    return `${value('year')}-${value('month')}-${value('day')}`;
  }
  return `${rule.slice(0,4)}-${rule.slice(4,6)}-${rule.slice(6,8)}`;
}
export const entryFormSchema = z.object({
  font_size: z.number().int().min(10, 'Use pelo menos 10 px.').max(24, 'Use at? 24 px.'),
  bold: z.boolean(), italic: z.boolean(), underline: z.boolean(),
  kind: z.enum(['task', 'event']), title: z.string().trim().min(1, 'Informe um título.').max(300, 'Use até 300 caracteres.'),
  description: z.string().max(100_000, 'Descrição muito longa.'), startDate: z.string().refine(validDate, 'Informe uma data válida.'),
  startTime: z.string(), endDate: z.string(), endTime: z.string(), allDay: z.boolean(),
  referenceStartUtc:z.string(), referenceEndUtc:z.string(), referenceScheduleKey:z.string(),
  categoryId: z.string(), priority: z.enum(['low', 'medium', 'high']), scope: z.enum(['one', 'future', 'series']),
  recurrence: z.enum(presets), frequency: z.enum(['DAILY', 'WEEKLY', 'MONTHLY', 'YEARLY']),
  interval: z.union([z.number(),z.nan()]),
  weekdays: z.array(z.enum(weekdays)), ending: z.enum(['never', 'until', 'count']), untilDate: z.string(),
  count: z.union([z.number(),z.nan()]),
  reminders: z.array(z.object({
    reminderId: z.string().optional(), preset: z.enum(['0','5','10','15','30','60','1440','custom','absolute']),
    lead: z.union([z.number(),z.nan()]), absolute: z.string(),
  })).max(12, 'Use até 12 lembretes.'),
}).superRefine((values, ctx) => {
  const issue = (path: (string | number)[], message: string) => ctx.addIssue({ code: 'custom', path, message });
  const timed = !values.allDay;
  if (values.scope !== 'one' && values.recurrence === 'custom' && (!Number.isInteger(values.interval)||values.interval<1||values.interval>999)) issue(['interval'],'Informe um intervalo inteiro entre 1 e 999.');
  if (values.scope !== 'one' && values.recurrence !== 'none' && values.ending === 'count' && (!Number.isInteger(values.count)||values.count<1||values.count>100_000)) issue(['count'],'Informe uma quantidade entre 1 e 100.000.');
  const timeValid = (value: string) => /^([01]\d|2[0-3]):[0-5]\d$/.test(value);
  if (timed && values.startTime && !timeValid(values.startTime)) issue(['startTime'], 'Horário inválido.');
  if (timed && values.endTime && !timeValid(values.endTime)) issue(['endTime'], 'Horário inválido.');
  if (values.endDate && !validDate(values.endDate)) issue(['endDate'], 'Informe uma data válida.');
  if (values.kind === 'event' && (!values.endDate || (timed && (!values.startTime || !values.endTime)))) issue(['endDate'], 'Informe o início e o término do evento.');
  if (timed && values.endTime && !values.startTime) issue(['startTime'], 'Informe o horário inicial.');
  if (values.endDate && values.endDate < values.startDate && !preservedInterval(values)) issue(['endDate'], 'O término não pode anteceder o início.');
  if (timed && values.startTime && values.endTime && (values.endDate || values.startDate) === values.startDate && values.endTime <= values.startTime && !preservedInterval(values)) issue(['endTime'], 'O término deve ser posterior ao início.');
  if (values.recurrence !== 'none' && values.scope !== 'one' && values.ending === 'until' && (!validDate(values.untilDate) || values.untilDate < values.startDate)) issue(['untilDate'], 'Informe uma data final igual ou posterior ao início.');
  const reminderKeys = new Set<string>();
  values.reminders.forEach((reminder, index) => {
    if (reminder.preset === 'custom' && (!Number.isInteger(reminder.lead)||reminder.lead<0||reminder.lead>525_600)) issue(['reminders',index,'lead'],'Informe uma antecedência inteira de até um ano.');
    if (reminder.preset === 'absolute' && (!reminder.absolute || Number.isNaN(new Date(reminder.absolute).getTime()))) issue(['reminders', index, 'absolute'], 'Informe uma data e horário válidos.');
    if (reminder.preset === 'absolute' && values.recurrence !== 'none' && values.scope !== 'one') issue(['reminders', index, 'absolute'], 'Em séries, use antecedência ou edite somente uma ocorrência.');
    const key = reminder.preset === 'absolute' ? `absolute:${new Date(reminder.absolute).getTime()}` : `relative:${reminder.preset === 'custom' ? reminder.lead : Number(reminder.preset)}`;
    if (reminderKeys.has(key)) issue(['reminders', index, 'preset'], 'Lembrete duplicado.');
    reminderKeys.add(key);
  });
});
export type EntryFormValues = z.infer<typeof entryFormSchema>;

export function recurrenceRule(values: EntryFormValues): string | null {
  if (values.recurrence === 'none') return null;
  const frequencies = { daily: 'DAILY', weekdays: 'DAILY', weekly: 'WEEKLY', monthly: 'MONTHLY', yearly: 'YEARLY', custom: values.frequency };
  const parts = [`FREQ=${frequencies[values.recurrence]}`, `INTERVAL=${values.recurrence === 'custom' ? values.interval : 1}`];
  if (values.recurrence === 'weekdays') parts.push('BYDAY=MO,TU,WE,TH,FR');
  else if (values.recurrence === 'custom' && values.weekdays.length) parts.push(`BYDAY=${values.weekdays.join(',')}`);
  if (values.ending === 'count') parts.push(`COUNT=${values.count}`);
  if (values.ending === 'until') parts.push(`UNTIL=${values.untilDate.replaceAll('-', '')}`);
  return parts.join(';');
}

export function toEntryDraft(values: EntryFormValues, timezone: string, individual = false): EntryDraft {
  const allDay = values.allDay || (values.kind === 'task' && !values.startTime);
  let endDate = values.endDate || (values.endTime && !allDay ? values.startDate : null);
  if (allDay && endDate) { const date = parseCivilDate(endDate); date.setDate(date.getDate() + 1); endDate = dateKey(date); }
  return {
    font_size: values.font_size, bold: values.bold, italic: values.italic, underline: values.underline,
    kind: values.kind, title: values.title.trim(), description: values.description, start_date: values.startDate,
    start_time: allDay ? null : values.startTime || null, end_date: endDate, end_time: allDay ? null : values.endTime || null,
    all_day: allDay, timezone, category_id: values.categoryId || null, priority: values.priority,
    recurrence_rule: individual ? null : recurrenceRule(values),
    start_utc:preservedInterval(values)?values.referenceStartUtc:undefined,
    end_utc:preservedInterval(values)?values.referenceEndUtc:undefined,
    duration_seconds:preservedInterval(values)?(Date.parse(values.referenceEndUtc)-Date.parse(values.referenceStartUtc))/1000:undefined,
    reminders: values.reminders.map((reminder) => ({ id: reminder.reminderId, kind: reminder.preset === 'absolute' ? 'absolute' : 'relative',
      minutes_before: reminder.preset === 'absolute' ? null : reminder.preset === 'custom' ? reminder.lead : Number(reminder.preset),
      scheduled_at: reminder.preset === 'absolute' ? new Date(reminder.absolute).toISOString() : null })),
  };
}

export function formDefaults(request: EditorRequest, reminders: ReminderDraft[] = []): EntryFormValues {
  const entry = request.entry; const start = request.start ?? new Date(); const end = request.end;
  const allDay = entry ? entry.all_day === 1 || entry.start_time === null : request.allDay ?? true;
  let endDate = entry?.end_date ?? (end ? dateKey(end) : '');
  if (allDay && endDate) { if (entry && endDate <= entry.start_date) endDate = entry.start_date; else {const date = parseCivilDate(endDate); date.setDate(date.getDate() - 1); endDate = dateKey(date);} }
  const rule = Object.fromEntries((entry?.recurrence_rule ?? '').split(';').filter(Boolean).map((part) => part.split('=')));
  const asLocalInput = (instant: string) => { const at = new Date(instant); return `${dateKey(at)}T${at.getHours().toString().padStart(2,'0')}:${at.getMinutes().toString().padStart(2,'0')}`; };
  const values:EntryFormValues = {
    font_size: entry?.font_size ?? 12, bold: entry?.bold ?? false, italic: entry?.italic ?? false, underline: entry?.underline ?? false,
    kind: entry?.kind ?? 'task', title: entry?.title ?? '', description: entry?.description ?? '', startDate: entry?.start_date ?? dateKey(start),
    startTime: entry?.start_time ?? (allDay ? '' : `${start.getHours().toString().padStart(2,'0')}:${start.getMinutes().toString().padStart(2,'0')}`),
    endDate, endTime: entry?.end_time ?? (end && !allDay ? `${end.getHours().toString().padStart(2,'0')}:${end.getMinutes().toString().padStart(2,'0')}` : ''), allDay,
    referenceStartUtc:entry?.start_time&&entry?.end_time?entry.start_utc??'':'',referenceEndUtc:entry?.start_time&&entry?.end_time?entry.end_utc??'':'',referenceScheduleKey:'',
    categoryId: entry?.category_id ?? '', priority: entry?.priority ?? 'medium', scope: entry?.recurrence_rule ? 'one' : 'series',
    recurrence: entry?.recurrence_rule ? 'custom' : 'none', frequency: (rule.FREQ ?? 'WEEKLY') as EntryFormValues['frequency'], interval: Number(rule.INTERVAL ?? 1),
    weekdays: (rule.BYDAY?.split(',') ?? []) as EntryFormValues['weekdays'], ending: rule.COUNT ? 'count' : rule.UNTIL ? 'until' : 'never',
    untilDate: untilDate(rule.UNTIL,entry?.timezone??Intl.DateTimeFormat().resolvedOptions().timeZone), count: Number(rule.COUNT ?? 10),
    reminders: reminders.map((reminder) => ({ reminderId: reminder.id ?? undefined, preset: reminder.kind === 'absolute' ? 'absolute' : ['0','5','10','15','30','60','1440'].includes(String(reminder.minutes_before)) ? String(reminder.minutes_before) as EntryFormValues['reminders'][number]['preset'] : 'custom', lead: reminder.minutes_before ?? 10, absolute: reminder.scheduled_at ? asLocalInput(reminder.scheduled_at) : '' })),
  };
  values.referenceScheduleKey=scheduleKey(values);return values;
}

export function entryIdentity(entry: CalendarEntry) {
  return { id: entry.series_id ?? entry.id, original: entry.original_start ?? `${entry.start_date}${entry.start_time ? `T${entry.start_time}` : ''}` };
}
