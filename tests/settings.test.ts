import { describe,it } from 'vitest';
import { settingsCases } from './settings.cases';
describe('Configurações',()=>{for(const sample of settingsCases)it(sample.name,sample.run);});
