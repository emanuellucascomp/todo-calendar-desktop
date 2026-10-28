import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource/roboto/latin-400.css';
import '@fontsource/roboto/latin-500.css';
import '@fontsource/roboto/latin-700.css';
import '@fontsource/roboto/latin-400-italic.css';
import '@fontsource/roboto/latin-700-italic.css';
import '@/styles/globals.css';
import { App } from '@/app/app';
import { z } from 'zod';

z.config(z.locales.pt());

const root = document.getElementById('root');
if (!root) throw new Error('Elemento raiz da aplicação não encontrado.');
createRoot(root).render(<StrictMode><App /></StrictMode>);
