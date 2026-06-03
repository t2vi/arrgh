import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.live.test.ts'],
    testTimeout: 60_000,
    hookTimeout: 30_000,
    // Never run in normal CI — these make real HTTP calls
    // Run with: npm test (from live-tests/) or vitest run src/<source>.live.test.ts
  },
})
