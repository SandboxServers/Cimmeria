# Vite 6 Configuration Patterns

> Guidelines for Vite build configuration and optimization. Apply when modifying build setup, adding plugins, or debugging build issues.

---

## Current Project Config

```typescript
// frontend/vite.config.ts
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
  },
  build: {
    target: 'esnext',
  },
});
```

> **Note:** After React migration is complete, the SolidJS plugin and `.react.tsx` file routing pattern will be removed.

## Plugin Configuration

### React Plugin

```typescript
import react from '@vitejs/plugin-react';

// Basic usage
react()

// With options
react({
  babel: {
    plugins: ['@emotion/babel-plugin'],
  },
})
```

### Tailwind CSS v4 (Vite Plugin)

```typescript
import tailwindcss from '@tailwindcss/vite';

// No configuration needed — Tailwind v4 uses CSS-native config
tailwindcss()
```

## Dev Server

### Proxy Configuration

For API development with a separate backend:

```typescript
server: {
  port: 5173,
  strictPort: true,
  proxy: {
    '/api': {
      target: 'http://localhost:3000',
      changeOrigin: true,
    },
    '/ws': {
      target: 'ws://localhost:3000',
      ws: true,
    },
  },
}
```

### HMR Configuration

```typescript
server: {
  hmr: {
    overlay: true,   // Show error overlay
    port: 5173,
  },
}
```

## Build Optimization

### Code Splitting

```typescript
build: {
  target: 'esnext',
  rollupOptions: {
    output: {
      manualChunks: {
        // Separate vendor chunks for better caching
        'react-vendor': ['react', 'react-dom'],
        'reactflow': ['@xyflow/react'],
        'three': ['three'],
      },
    },
  },
}
```

### Bundle Analysis

```bash
# Install analyzer
npm install -D rollup-plugin-visualizer

# Add to vite.config.ts
import { visualizer } from 'rollup-plugin-visualizer';

plugins: [
  // ... other plugins
  visualizer({ open: true, gzipSize: true }),
]
```

### Chunk Size Warnings

```typescript
build: {
  chunkSizeWarningLimit: 500, // KB
}
```

## Environment Variables

```typescript
// .env files
// .env              — always loaded
// .env.local        — always loaded, git-ignored
// .env.development  — dev mode only
// .env.production   — build mode only

// Access in code (must be prefixed with VITE_)
const apiUrl = import.meta.env.VITE_API_URL;
const isProd = import.meta.env.PROD;
const isDev = import.meta.env.DEV;
```

### Type-Safe Env Variables

```typescript
// src/env.d.ts
/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string;
  readonly VITE_WS_URL: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
```

## Path Aliases

```typescript
import { resolve } from 'path';

export default defineConfig({
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@components': resolve(__dirname, 'src/components'),
      '@lib': resolve(__dirname, 'src/lib'),
    },
  },
});
```

Update `tsconfig.json` to match:
```json
{
  "compilerOptions": {
    "paths": {
      "@/*": ["./src/*"],
      "@components/*": ["./src/components/*"],
      "@lib/*": ["./src/lib/*"]
    }
  }
}
```

## Testing with Vitest

Vitest shares Vite's config by default:

```typescript
// vitest.config.ts (or inline in vite.config.ts)
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    css: true,
    setupFiles: './src/test/setup.ts',
  },
});
```

## Common Issues

### Build Errors

| Issue | Solution |
|-------|----------|
| Module not found | Check `resolve.alias` and `tsconfig.json` paths match |
| JSX transform error | Ensure correct plugin (react vs solid) |
| Large bundle | Use `manualChunks` to split vendor code |
| HMR not working | Check WebSocket connection, try `--force` flag |
| CSS not loading | Verify Tailwind plugin is in `plugins` array |

### Performance Tips

1. Use `build.target: 'esnext'` for modern browsers (skip polyfills)
2. Enable `build.sourcemap: false` in production for smaller output
3. Use dynamic `import()` for route-level code splitting
4. Exclude large dependencies from pre-bundling if causing issues

## Project Commands

```bash
cd frontend
npm run dev       # Start dev server (port 5173)
npm run build     # Build for production (runs tests first)
npm run preview   # Preview production build locally
```
