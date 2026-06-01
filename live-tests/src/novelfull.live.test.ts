import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import * as plugin from '../../plugins/novelfull/src/index'
import { connectBrowser, type BrowserConn } from './helpers'
import corpus from '../corpus/novelfull.json'

let conn: BrowserConn | null = null

beforeAll(async () => { conn = await connectBrowser() })
afterAll(async () => { await conn?.close() })

describe('novelfull — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async ({ skip }) => {
      if (!conn) skip()

      plugin.init(conn!.makeContext('novelfull', 'search', query))
      const results = await plugin.search(query)
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      plugin.init(conn!.makeContext('novelfull', 'meta', query))
      const meta = await plugin.meta(results[0].id)
      expect(meta).toMatchSnapshot()

      plugin.init(conn!.makeContext('novelfull', 'chapters', query))
      const chapters = await plugin.chapters(results[0].id)
      expect(chapters).toMatchSnapshot()

      if (chapters.length === 0) return

      plugin.init(conn!.makeContext('novelfull', 'chapterText', query))
      const text = await plugin.chapterText(chapters[chapters.length - 1].source_id)
      expect(text).toMatchSnapshot()
    })
  }
})
