import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
  ],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://localhost:8443',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:8443',
        ws: true,
      },
    },
  },
  build: {
    target: 'esnext',
  },
});
