import { describe, it, expect, beforeAll } from 'vitest'
import { getToken, apiGet } from './helpers'
import corpus from '../corpus/discover.json'

interface DiscoverResult {
  title: string
  content_type: string
  source: string
  author: string | null
  status: string
  year: number | null
  is_explicit: boolean
  in_library: boolean
}

let token: string
beforeAll(async () => { token = await getToken() })

for (const [contentType, queries] of Object.entries(corpus) as [string, string[]][]) {
  describe(`discover — ${contentType}`, () => {
    for (const q of queries) {
      it(`search: ${q}`, async () => {
        const results = await apiGet<DiscoverResult[]>(
          `/api/discover?q=${encodeURIComponent(q)}`,
          token,
        )
        // Snapshot stable fields — cover_url/description excluded (CDN urls + text change often)
        const stable = results.map(({ title, content_type, source, author, status, year, is_explicit }) => ({
          title, content_type, source, author, status, year, is_explicit,
        }))
        expect(stable).toMatchSnapshot()
      })
    }
  })
}
