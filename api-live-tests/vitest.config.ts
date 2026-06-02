import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.live.test.ts'],
    testTimeout: 180_000,
    hookTimeout: 30_000,
    // Sequential + single worker: CloakBrowser is single-process; concurrent nhentai
    // requests from multiple workers causes empty chapter responses.
    fileParallelism: false,
    pool: 'forks',
    poolOptions: { forks: { maxForks: 1 } },
    // Never run in normal CI — requires a running API server (localhost:3000 by default)
    // Run with: npm test (from api-live-tests/) — or set API_URL to target a different server
  },
})
