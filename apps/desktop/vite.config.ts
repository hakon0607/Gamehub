import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// Tauri serves the built files from disk in release and from this dev server in
// development. The fixed port matters: it is written into tauri.conf.json.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'chrome110', sourcemap: false },
});
