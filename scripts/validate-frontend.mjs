import { build } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { validationConfig } from './validation-config.mjs';
const config = validationConfig({ production: true });
config.plugins.push(tailwindcss());
await build({ ...config, build: { target: 'esnext', minify: false, cssMinify: 'lightningcss', outDir: '.validation-dist', emptyOutDir: true,
  rollupOptions: { input: { app: 'index.html', editor: '.qa/editor.html' } },
} });
