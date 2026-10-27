import { preview } from 'vite';
import { validationConfig } from './validation-config.mjs';
const server = await preview({ ...validationConfig(), build: { outDir: '.validation-dist' }, preview: { host: '127.0.0.1', port: 4173, strictPort: true, open: false } });
server.printUrls();
