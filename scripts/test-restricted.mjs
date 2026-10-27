import { startVitest } from 'vitest/node';
import { validationConfig } from './validation-config.mjs';
const config = validationConfig();
const context = await startVitest('test', [], {
  run: true, watch: false, config: false, pool: 'threads', maxWorkers: 1, minWorkers: 1,
  include: ['tests/**/*.test.ts'], exclude: ['tests/**/*.node.test.ts'],
}, config);
if (!context) process.exitCode = 1;
else { if (context.state.getFiles().length === 0 || context.state.getUnhandledErrors().length > 0 || context.state.getFiles().some((file) => file.result?.state !== 'pass')) process.exitCode = 1; await context.close(); }
