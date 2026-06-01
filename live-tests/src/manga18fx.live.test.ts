import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import * as plugin from '../../plugins/manga18fx/src/index'
import { connectBrowser, type BrowserConn } from './helpers'
import corpus from '../corpus/manga18fx.json'

let conn: BrowserConn | null = null

beforeAll(async () => { conn = await connectBrowser() })
afterAll(async () => { await conn?.close() })

describe('manga18fx — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async ({ skip }) => {
      if (!conn) skip()

      plugin.init(conn!.makeContext('manga18fx', 'search', query))
      const results = await plugin.search(query)
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      plugin.init(conn!.makeContext('manga18fx', 'chapters', query))
      const chapters = await plugin.chapters(results[0].id)
      expect(chapters).toMatchSnapshot()

      if (chapters.length === 0) return

      plugin.init(conn!.makeContext('manga18fx', 'pages', query))
      const pages = await plugin.pages(chapters[0].source_id)
      expect(pages).toMatchSnapshot()
    })
  }
})
