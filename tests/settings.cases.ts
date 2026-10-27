import assert from 'node:assert/strict';
import { preferencesSchema } from '../src/features/settings/schema.ts';
import { DEFAULT_PREFERENCES } from '../src/types/settings.ts';
export const settingsCases: {name:string;run:()=>void}[] = [
  {name:'preferências começam com autostart desativado e tema do sistema',run(){const settings=preferencesSchema.parse(DEFAULT_PREFERENCES);assert.equal(settings.autostart,false);assert.equal(settings.theme,'system');}},
  {name:'configurações rejeitam primeiro dia, tema e antecedência inválidos',run(){for(const change of [{first_day:7},{theme:'unknown'},{default_reminder_minutes:-1}])assert.equal(preferencesSchema.safeParse({...DEFAULT_PREFERENCES,...change}).success,false);}},
  {name:'configurações aceitam tema escuro, segunda-feira e fechamento na bandeja',run(){const settings=preferencesSchema.parse({...DEFAULT_PREFERENCES,theme:'dark',first_day:1,close_behavior:'tray'});assert.equal(settings.first_day,1);assert.equal(settings.close_behavior,'tray');}},
];
