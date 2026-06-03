import { describe, it, expect } from 'vitest'
import * as plugin from '../../plugins/wuxiaworld/src/index'
import { captureFetch } from './helpers'
import corpus from '../corpus/wuxiaworld.json'

describe('wuxiaworld — live snapshots', () => {
  for (const query of corpus.search) {
    it(`search: ${query}`, async () => {
      const results = await captureFetch('wuxiaworld', 'search', query, () => plugin.search(query))
      expect(results).toMatchSnapshot()

      if (results.length === 0) return

      const first = results[0]
      const chapters = await captureFetch('wuxiaworld', 'chapters', query,
        () => plugin.chapters(first.id))
      expect(chapters).toMatchSnapshot()

      if (chapters.length === 0) return

      const text = await captureFetch('wuxiaworld', 'chapterText', query,
        () => plugin.chapterText(chapters[chapters.length - 1].source_id))
      expect(text).toMatchSnapshot()
    })
  }
})
