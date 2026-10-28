import { z } from 'zod';
const color = z.string().regex(/^#[0-9a-fA-F]{6}$/, 'Use uma cor hexadecimal com seis dÃ­gitos.').nullable();
export const preferencesSchema = z.object({
  calendar_text_color: color, calendar_background_color: color,
  calendar_font_size: z.number().int().min(10, 'Use pelo menos 10 px.').max(24, 'Use até 24 px.'),
  calendar_bold: z.boolean(), calendar_italic: z.boolean(), calendar_underline: z.boolean(),
  theme: z.enum(['light','dark','system']), first_day: z.union([z.literal(0),z.literal(1)]),
  default_view: z.enum(['month','week','week-grid','year']), show_completed: z.boolean(), notifications_enabled: z.boolean(),
  default_reminder_minutes: z.number().int('Informe um nÃºmero inteiro.').min(0,'Use zero ou mais minutos.').max(525_600,'Use atÃ© um ano.'),
  autostart: z.boolean(), start_in_tray: z.boolean(), minimize_to_tray: z.boolean(), close_behavior: z.enum(['exit','tray']),
});
