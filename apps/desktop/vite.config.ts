import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Tauri serves the built files from disk in release and from this dev server in
// development. The fixed port matters: it is written into tauri.conf.json.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  // The legal documents live in /legal at the top of the repository so there
  // is only ever one copy of each; the dev server has to be allowed to read
  // them from outside this folder. The production build inlines them anyway.
  server: { port: 1420, strictPort: true, fs: { allow: ['..', '../../legal'] } },
  build: { target: 'chrome110', sourcemap: false },
});
