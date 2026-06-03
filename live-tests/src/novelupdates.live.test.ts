import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import * as plugin from '../../plugins/novelupdates/src/index'
import { connectBrowser, type BrowserConn } from './helpers'
import corpus from '../corpus/novelupdates.json'

let conn: BrowserConn | null = null

beforeAll(async () => { conn = await connectBrowser() })
afterAll(async () => { await conn?.close() })

describe('novelupdates — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async ({ skip }) => {
      if (!conn) skip()

      plugin.init(conn!.makeContext('novelupdates', 'search', query))
      const results = await plugin.search(query)
      expect(results.length).toBeGreaterThan(0)
      expect(results).toMatchSnapshot()

      // novelupdates.chapters() always returns [] — it's a discovery-only source
      // No chapter/page chain needed
    })
  }
})
