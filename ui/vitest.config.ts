import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  // vitest исполняет модули в SSR-контексте; для mount() нужен клиентский
  // бандл Svelte (@testing-library/svelte).
  resolve: {
    conditions: ['browser'],
  },
  test: {
    environment: 'node',
    // Компонентные тесты помечают jsdom через docblock `@vitest-environment`.
    environmentOptions: {
      jsdom: {
        // rAF для пружины иглы в CentsGauge.
        pretendToBeVisual: true,
      },
    },
    include: ['tests/**/*.test.ts'],
    setupFiles: ['tests/setup.ts'],
  },
});
