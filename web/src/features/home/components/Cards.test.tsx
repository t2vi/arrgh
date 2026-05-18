import { render, screen } from '@testing-library/react'
import { vi, describe, it, expect } from 'vitest'
import { TrendingCard } from './Cards'

vi.mock('@/api', () => ({
  api: {
    proxyImageUrl: (url: string) =>
      url.startsWith('/api/') ? url : `/api/media/proxy?url=${encodeURIComponent(url)}`,
  },
}))

function makeResult(overrides: object = {}) {
  return {
    id: 't1', source: 'mangadex', source_name: 'MangaDex', title: 'Berserk',
    description: null, cover_url: null, status: 'ongoing',
    author: null, year: null, tags: null, content_type: 'manga',
    in_library: false, library_id: undefined, alternatives: [],
    ...overrides,
  }
}

describe('TrendingCard', () => {
  it('renders skeleton pulse when cover_url is null', () => {
    const { container } = render(
      <TrendingCard result={makeResult({ cover_url: null })} badge="HOT" onClick={() => {}} />
    )
    expect(container.querySelector('.animate-pulse')).toBeTruthy()
    expect(screen.queryByRole('img')).toBeNull()
  })

  it('renders img when cover_url is set', () => {
    const { container } = render(
      <TrendingCard
        result={makeResult({ cover_url: 'https://cdn.example.com/cover.jpg' })}
        badge="TOP"
        onClick={() => {}}
      />
    )
    const img = container.querySelector('img')
    expect(img).toBeTruthy()
    expect(img!.src).toContain('proxy')
  })

  it('renders /api/ cover_url directly without double-proxying', () => {
    const { container } = render(
      <TrendingCard
        result={makeResult({ cover_url: '/api/media/meta-cover?key=berserk' })}
        badge="HOT"
        onClick={() => {}}
      />
    )
    const img = container.querySelector('img')
    expect(img).toBeTruthy()
    expect(img!.src).toContain('/api/media/meta-cover')
    expect(img!.src).not.toContain('proxy')
  })

  it('shows book emoji on img error not skeleton', async () => {
    const { container } = render(
      <TrendingCard
        result={makeResult({ cover_url: 'https://cdn.example.com/broken.jpg' })}
        badge="HOT"
        onClick={() => {}}
      />
    )
    const img = container.querySelector('img')!
    img.dispatchEvent(new Event('error', { bubbles: true }))

    await vi.waitFor(() => {
      expect(container.querySelector('img')).toBeNull()
      expect(container.querySelector('.animate-pulse')).toBeNull()
      expect(screen.getByText('📖')).toBeTruthy()
    })
  })

  it('shows title and source_name below cover', () => {
    render(
      <TrendingCard result={makeResult()} badge="NEW" onClick={() => {}} />
    )
    expect(screen.getByText('Berserk')).toBeTruthy()
    expect(screen.getByText('MangaDex')).toBeTruthy()
  })

  it('shows badge', () => {
    render(
      <TrendingCard result={makeResult()} badge="TOP" onClick={() => {}} />
    )
    expect(screen.getByText('TOP')).toBeTruthy()
  })
})
