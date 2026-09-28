import { createServer } from 'node:net'
import { describe, expect, it } from 'vitest'
import { PORT } from './index'

// spec: 025/FR-001 — importing the host module (as every other test file in this
// suite does) must never bind a real listener on PORT; if it did, this bind would
// fail with EADDRINUSE, exactly the bug this spec fixed (GH: npm test failing while
// ./scripts/dev-up.sh already holds :4000).
describe('importing the plugin-host module does not start a real server', () => {
  it(`nothing is already listening on :${PORT} after import`, async () => {
    await new Promise<void>((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(PORT, () => {
        server.close(() => resolve())
      })
    })
  })
})
