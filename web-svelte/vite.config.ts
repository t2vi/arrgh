import { defineConfig } from 'vitest/config'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

const dirname = import.meta.dirname

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(dirname, './src'),
    },
    conditions: process.env.VITEST ? ['browser'] : undefined,
  },
  server: {
    proxy: {
      '/api': 'http://localhost:3001',
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['allure-vitest/setup', './src/test-setup.ts'],
    reporters: process.env.CI
      ? [['allure-vitest/reporter', { resultsDir: './allure-results' }], 'verbose']
      : ['verbose'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      reportsDirectory: './coverage',
      exclude: ['src/test-setup.ts', 'src/lib/components/ui/**'],
    },
  },
})
