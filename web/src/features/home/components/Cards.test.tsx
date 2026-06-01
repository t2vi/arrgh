import { render, screen } from '@testing-library/react'
import { vi, describe, it, expect } from 'vitest'
import { TrendingCard, LibraryCoverCard } from './Cards'
import type { Title } from '@/types'

vi.mock('@/api', () => ({
  api: {
    proxyImageUrl: (url: string) =>
      url.startsWith('/api/') ? url : `/api/media/proxy?url=${encodeURIComponent(url)}`,
    coverUrl: (id: string) => `/covers/${id}`,
  },
}))

function makeResult(overrides: object = {}) {
  return {
    mangaupdates_id: 't1', title: 'Berserk',
    description: null, cover_url: null, status: 'ongoing',
    author: 'Miura, Kentaro', year: 1989, tags: null, content_type: 'manga',
    in_library: false, library_id: null,
    is_explicit: false,
    source: 'mangaupdates',
    ...overrides,
  }
}

function makeManga(overrides: Partial<Title> = {}): Title {
  return {
    id: 'm1', title: 'Berserk', author: 'Miura', description: null,
    cover_url: null, status: 'ongoing', content_type: 'manga',
    sync_status: 'ready', is_local: false, local_path: null,
    year: null, tags: null, auto_download: null, reader_mode: null,
    download_dir: null, is_explicit: false, has_sync_warnings: false,
    created_at: '', updated_at: '',
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

  it('shows title and author below cover', () => {
    render(
      <TrendingCard result={makeResult()} badge="NEW" onClick={() => {}} />
    )
    expect(screen.getByText('Berserk')).toBeTruthy()
    expect(screen.getByText('Miura, Kentaro')).toBeTruthy()
  })

  it('shows badge', () => {
    render(
      <TrendingCard result={makeResult()} badge="TOP" onClick={() => {}} />
    )
    expect(screen.getByText('TOP')).toBeTruthy()
  })

  it('shows 18+ pill when is_explicit is true', () => {
    render(<TrendingCard result={makeResult({ is_explicit: true })} badge="HOT" onClick={() => {}} />)
    expect(screen.getByText('18+')).toBeInTheDocument()
  })

  it('does not show 18+ pill when is_explicit is false', () => {
    render(<TrendingCard result={makeResult({ is_explicit: false })} badge="HOT" onClick={() => {}} />)
    expect(screen.queryByText('18+')).not.toBeInTheDocument()
  })
})

describe('LibraryCoverCard', () => {
  it('shows 18+ pill when is_explicit is true', () => {
    render(<LibraryCoverCard manga={makeManga({ is_explicit: true })} onClick={() => {}} />)
    expect(screen.getByText('18+')).toBeInTheDocument()
  })

  it('does not show 18+ pill when is_explicit is false', () => {
    render(<LibraryCoverCard manga={makeManga({ is_explicit: false })} onClick={() => {}} />)
    expect(screen.queryByText('18+')).not.toBeInTheDocument()
  })
})
