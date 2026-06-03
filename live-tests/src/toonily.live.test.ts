import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import * as plugin from '../../plugins/toonily/src/index'
import { connectBrowser, type BrowserConn } from './helpers'
import corpus from '../corpus/toonily.json'

let conn: BrowserConn | null = null

beforeAll(async () => { conn = await connectBrowser() })
afterAll(async () => { await conn?.close() })

describe('toonily — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async ({ skip }) => {
      if (!conn) skip()

      plugin.init(conn!.makeContext('toonily', 'search', query))
      const results = await plugin.search(query)
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      const first = results[0]
      plugin.init(conn!.makeContext('toonily', 'chapters', query))
      const chapters = await plugin.chapters(first.id)
      expect(chapters).toMatchSnapshot()

      if (chapters.length === 0) return

      plugin.init(conn!.makeContext('toonily', 'pages', query))
      const pages = await plugin.pages(chapters[0].source_id)
      expect(pages).toMatchSnapshot()
    })
  }
})
