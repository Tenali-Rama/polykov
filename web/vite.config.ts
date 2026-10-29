import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

const server = 'http://127.0.0.1:7700';

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  // Assets are referenced from `/` so deep SPA paths still load them.
  base: '/',
  build: { outDir: 'dist', emptyOutDir: true },
  server: {
    proxy: {
      '/health': server,
      '/api': server,
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
  },
});
