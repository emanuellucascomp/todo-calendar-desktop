// Somente revisão visual: nenhuma substituição do SQLite e nenhum registro é salvo.
import { createRoot } from 'react-dom/client';
import { EntryEditor } from '../src/components/entries/entry-editor';
import '../src/styles/globals.css';
import '@fontsource/roboto/latin-400.css';
import '@fontsource/roboto/latin-500.css';
import '@fontsource/roboto/latin-700.css';
import { useEffect } from 'react';
import { useCalendarStore } from '../src/stores/calendar-store';

function Preview() {
  const request=useCalendarStore((state)=>state.editorRequest);
  useEffect(()=>{useCalendarStore.getState().openCreate({start:new Date(),allDay:true});},[]);
  return <main style={{padding:32}}><h1>Revisão visual do formulário · sem persistência</h1><button type="button" onClick={()=>useCalendarStore.getState().openCreate()}>Abrir formulário</button>{request&&<EntryEditor request={request} categories={[]} defaultReminder={10}/>}</main>;
}
const root=document.getElementById('root');if(root)createRoot(root).render(<Preview/>);
