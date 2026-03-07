import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import solidPlugin from 'vite-plugin-solid';
import tailwindcss from '@tailwindcss/vite';

const reactIslandRE = /\.react\.[tj]sx$/;

export default defineConfig({
  plugins: [
    react({ include: reactIslandRE }),
    solidPlugin({ exclude: reactIslandRE }),
    tailwindcss(),
  ],
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: 'esnext',
  },
});
