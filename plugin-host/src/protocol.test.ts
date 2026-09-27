// Protocol-drift guard (spec 031 phase D): plugin-host's own PLUGIN_PROTOCOL must never
// silently diverge from the shared SDK's — a plugin repo builds against the SDK's constant,
// this host enforces its own copy at install time (index.ts's `protocol > PLUGIN_PROTOCOL`
// check). plugin-host stays CommonJS (its dynamic `require()`-based bundle loading can't take
// a runtime dependency on an ESM-only package), so this is a dev-only, test-time check via a
// dynamic import — not a production dependency.
import { describe, it, expect } from 'vitest'
import { PLUGIN_PROTOCOL } from './index'

describe('plugin protocol matches the SDK (spec 031 phase D)', () => {
  it('PLUGIN_PROTOCOL equals arrgh-plugin-sdk\'s', async () => {
    const sdk = await import('arrgh-plugin-sdk')
    expect(PLUGIN_PROTOCOL).toBe(sdk.PLUGIN_PROTOCOL)
  })
})
