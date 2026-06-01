import { describe, it, expect } from 'vitest'
import * as plugin from '../../plugins/mangapill/src/index'
import { captureFetch } from './helpers'
import corpus from '../corpus/mangapill.json'

describe('mangapill — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async () => {
      const results = await captureFetch('mangapill', 'search', query, () => plugin.search(query))
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      const first = results[0]
      const chapters = await captureFetch('mangapill', 'chapters', query,
        () => plugin.chapters(first.id))
      expect(chapters).toMatchSnapshot()

      if (chapters.length === 0) return

      const pages = await captureFetch('mangapill', 'pages', query,
        () => plugin.pages(chapters[0].source_id))
      expect(pages).toMatchSnapshot()
    })
  }
})
