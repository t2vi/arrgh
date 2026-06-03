import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest'
import * as plugin from '../../plugins/nhentai/src/index'
import { connectBrowser, type BrowserConn } from './helpers'
import corpus from '../corpus/nhentai.json'

let conn: BrowserConn | null = null

beforeAll(async () => { conn = await connectBrowser() })
afterAll(async () => { await conn?.close() })
// nhentai rate-limits aggressively — wait between each test
beforeEach(async () => { await new Promise(r => setTimeout(r, 30000)) })

describe('nhentai — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async ({ skip }) => {
      if (!conn) skip()

      plugin.init(conn!.makeContext('nhentai', 'search', query))
      const results = await plugin.search(query)
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      plugin.init(conn!.makeContext('nhentai', 'chapters', query))
      const chapters = await plugin.chapters(results[0].id)
      expect(chapters).toMatchSnapshot()

      if (chapters.length === 0) return

      plugin.init(conn!.makeContext('nhentai', 'pages', query))
      const pages = await plugin.pages(chapters[0].source_id)
      expect(pages).toMatchSnapshot()
    })
  }
})
