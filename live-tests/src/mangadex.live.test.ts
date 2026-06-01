import { describe, it, expect } from 'vitest'
import * as plugin from '../../plugins/mangadex/src/index'
import { captureFetch } from './helpers'
import corpus from '../corpus/mangadex.json'

describe('mangadex — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async () => {
      const results = await captureFetch('mangadex', 'search', query, () => plugin.search(query))
      // Snapshot whatever the source returns — 0 results is valid data (title not found/indexed)
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      const first = results[0]
      const chapters = await captureFetch('mangadex', 'chapters', query,
        () => plugin.chapters(first.id, ['en']))
      expect(chapters).toMatchSnapshot()

      // Skip pages if no English chapters (e.g. licensed titles with externalUrl)
      if (chapters.length === 0) return

      const pages = await captureFetch('mangadex', 'pages', query,
        () => plugin.pages(chapters[0].source_id))
      expect(pages).toMatchSnapshot()
    })
  }
})
