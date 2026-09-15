import { describe, expect, it } from 'vitest'
import { matchPath } from './router.svelte'

describe('matchPath', () => {
  it('matches a static path', () => {
    expect(matchPath('/library', '/library')).toEqual({})
  })

  it('returns null on a non-matching static path', () => {
    expect(matchPath('/library', '/discover')).toBeNull()
  })

  it('captures a dynamic segment', () => {
    expect(matchPath('/title/:id', '/title/abc-123')).toEqual({ id: 'abc-123' })
  })

  it('decodes a dynamic segment', () => {
    expect(matchPath('/title/:id', '/title/a%20b')).toEqual({ id: 'a b' })
  })

  it('rejects mismatched segment counts', () => {
    expect(matchPath('/title/:id', '/title/abc/extra')).toBeNull()
  })
})
