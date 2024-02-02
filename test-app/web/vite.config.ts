import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:7823',
        changeOrigin: true,
      },
    },
  },
  test: {
    include: ['src/**/*.{test,spec}.{js,ts}'],
  },
});
