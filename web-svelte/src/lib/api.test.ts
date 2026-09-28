import { afterEach, describe, expect, it, vi } from 'vitest'
import { api, getToken, setToken, splitNdjson } from './api'

describe('splitNdjson', () => {
  it('parses every complete line in a chunk', () => {
    const { lines, rest } = splitNdjson('{"a":1}\n{"b":2}\n')
    expect(lines).toEqual([{ a: 1 }, { b: 2 }])
    expect(rest).toBe('')
  })

  it('holds a trailing partial line until the next chunk completes it', () => {
    const first = splitNdjson('{"a":1}\n{"b"')
    expect(first.lines).toEqual([{ a: 1 }])
    expect(first.rest).toBe('{"b"')
    const second = splitNdjson(first.rest + ':2}\n')
    expect(second.lines).toEqual([{ b: 2 }])
    expect(second.rest).toBe('')
  })

  it('ignores blank lines', () => {
    expect(splitNdjson('\n{"a":1}\n\n').lines).toEqual([{ a: 1 }])
  })
})

// spec: 003/FR-006, 003/FR-007
describe('401 handling', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    localStorage.clear()
  })

  it('clears the stored token and dispatches arrgh:unauthorized on a 401 response', async () => {
    setToken('tok', 'user', 'member', false)
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: async () => ({}),
      }),
    )
    const listener = vi.fn()
    window.addEventListener('arrgh:unauthorized', listener)

    await expect(api.getSettings()).rejects.toThrow()

    expect(getToken()).toBeNull()
    expect(listener).toHaveBeenCalledTimes(1)
    window.removeEventListener('arrgh:unauthorized', listener)
  })
})
