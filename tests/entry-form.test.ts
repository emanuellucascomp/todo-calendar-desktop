import { describe, it } from 'vitest';
import { formCases } from './entry-form.cases';
describe('Formulários de tarefas e eventos',()=>{for(const sample of formCases)it(sample.name,sample.run);});
