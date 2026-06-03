import { describe, it, expect, beforeAll } from 'vitest'
import { getToken, apiGet } from './helpers'

interface SourceRow {
  id: string
  name: string
  content_types: string[]
  enabled: boolean
  priority: number
  is_community: boolean
}

let token: string
beforeAll(async () => { token = await getToken() })

describe('sources', () => {
  it('list matches snapshot', async () => {
    const sources = await apiGet<SourceRow[]>('/api/sources', token)
    // Snapshot stable fields only — base_url varies by deployment
    const stable = sources.map(({ name, content_types, enabled, priority }) => ({
      name, content_types, enabled, priority,
    }))
    expect(stable).toMatchSnapshot()
  })

  it('nhentai content_types is ["hentai"] not ["manga"]', async () => {
    const sources = await apiGet<SourceRow[]>('/api/sources', token)
    const nhentai = sources.find(s => s.name === 'nhentai')
    expect(nhentai).toBeDefined()
    expect(nhentai!.content_types).toContain('hentai')
    expect(nhentai!.content_types).not.toContain('manga')
  })

  it('removed plugins absent from sources', async () => {
    const sources = await apiGet<SourceRow[]>('/api/sources', token)
    const keys = sources.map(s => s.name.toLowerCase().replace(/\s+/g, ''))
    expect(keys).not.toContain('royalroad')
    expect(keys).not.toContain('manhuafast')
    expect(keys).not.toContain('boxnovel')
  })
})
