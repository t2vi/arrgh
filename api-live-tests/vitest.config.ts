import { defineConfig } from 'vitest/config'
import { BaseSequencer } from 'vitest/node'

// Respect CLI file order — vitest's default performance sequencer reorders by prior duration,
// which breaks the required nhentai-first ordering (CB-dependent tests must run before
// long-running novel/manhwa syncs that can strain the plugin-host).
class CliOrderSequencer extends BaseSequencer {
  override async sort(files: { file: string }[]) {
    return files
  }
}

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['src/**/*.live.test.ts'],
    testTimeout: 180_000,
    hookTimeout: 180_000,
    sequence: {
      sequencer: CliOrderSequencer,
    },
    // Sequential + single worker: CloakBrowser is single-process; concurrent nhentai
    // requests from multiple workers causes empty chapter responses.
    fileParallelism: false,
    pool: 'forks',
    poolOptions: { forks: { maxForks: 1 } },
    // Never run in normal CI — requires a running API server (localhost:3000 by default)
    // Run with: npm test (from api-live-tests/) — or set API_URL to target a different server
  },
})
