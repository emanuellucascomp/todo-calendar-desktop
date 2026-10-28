import assert from 'node:assert/strict';
import { preferencesSchema } from '../src/features/settings/schema.ts';
import { DEFAULT_PREFERENCES } from '../src/types/settings.ts';
export const settingsCases: {name:string;run:()=>void}[] = [
  {name:'cores personalizadas aceitam apenas hexadecimal completo e permitem restaurar o tema',run(){
    assert.equal(preferencesSchema.parse({...DEFAULT_PREFERENCES,calendar_text_color:'#aBc123',calendar_background_color:'#102030'}).calendar_text_color,'#aBc123');
    assert.equal(preferencesSchema.parse(DEFAULT_PREFERENCES).calendar_background_color,null);
    for(const color of ['red','#fff','#12345g','url(test)','']) {
      for(const field of ['calendar_text_color','calendar_background_color']) assert.equal(preferencesSchema.safeParse({...DEFAULT_PREFERENCES,[field]:color}).success,false);
    }
  }},
  {name:'preferências começam com autostart desativado e tema do sistema',run(){const settings=preferencesSchema.parse(DEFAULT_PREFERENCES);assert.equal(settings.autostart,false);assert.equal(settings.theme,'system');}},
  {name:'configurações rejeitam primeiro dia, tema e antecedência inválidos',run(){for(const change of [{calendar_font_size:9},{calendar_font_size:25},{calendar_font_size:12.5},{calendar_bold:'true'},{first_day:7},{theme:'unknown'},{default_reminder_minutes:-1}])assert.equal(preferencesSchema.safeParse({...DEFAULT_PREFERENCES,...change}).success,false);}},
  {name:'configurações aceitam tema escuro, segunda-feira e fechamento na bandeja',run(){const settings=preferencesSchema.parse({...DEFAULT_PREFERENCES,theme:'dark',first_day:1,close_behavior:'tray'});assert.equal(settings.first_day,1);assert.equal(settings.close_behavior,'tray');}},
];
