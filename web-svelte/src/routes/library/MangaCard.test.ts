import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

vi.mock('../../lib/api', () => ({
  api: { coverUrl: (id: string) => `/covers/${id}` },
}))

import MangaCard from './MangaCard.svelte'
import type { Title } from '../../lib/types'

function makeManga(overrides: Partial<Title> = {}): Title {
  return {
    id: 'm1',
    title: 'Vinland Saga',
    author: 'Makoto Yukimura',
    description: null,
    cover_url: null,
    status: 'ongoing',
    content_type: 'manga',
    sync_status: 'ready',
    is_local: false,
    local_path: null,
    year: null,
    tags: null,
    auto_download: null,
    reader_mode: null,
    download_dir: null,
    is_explicit: false,
    has_sync_warnings: false,
    total_chapters: 10,
    downloaded_chapters: 5,
    chapters_read: 3,
    created_at: '',
    updated_at: '',
    ...overrides,
  }
}

function baseProps(overrides: Record<string, unknown> = {}) {
  return { manga: makeManga(), onClick: () => {}, onRemove: () => {}, isRemoving: false, ...overrides }
}

describe('MangaCard', () => {
  it('renders title and author', () => {
    render(MangaCard, baseProps())
    expect(screen.getByText('Vinland Saga')).toBeInTheDocument()
    expect(screen.getByText('Makoto Yukimura')).toBeInTheDocument()
  })

  // The outer card wrapper is also role="button" (a11y fix for the clickable
  // div), so scope to the actual <button> element for the trash control.
  it('shows confirm dialog after trash button click', async () => {
    const { container } = render(MangaCard, baseProps())
    const trashBtn = container.querySelector('button')!
    await userEvent.click(trashBtn)
    expect(screen.getByText('Remove title?')).toBeInTheDocument()
  })

  it('calls onRemove(false) from "Library only" confirm', async () => {
    const onRemove = vi.fn()
    const { container } = render(MangaCard, baseProps({ onRemove }))
    await userEvent.click(container.querySelector('button')!)
    await userEvent.click(screen.getByText('Library only'))
    expect(onRemove).toHaveBeenCalledWith(false)
  })

  it('calls onRemove(true) from "Remove + delete files" confirm', async () => {
    const onRemove = vi.fn()
    const { container } = render(MangaCard, baseProps({ onRemove }))
    await userEvent.click(container.querySelector('button')!)
    await userEvent.click(screen.getByText('Remove + delete files'))
    expect(onRemove).toHaveBeenCalledWith(true)
  })

  it('cancel closes the confirm dialog', async () => {
    const { container } = render(MangaCard, baseProps())
    await userEvent.click(container.querySelector('button')!)
    await userEvent.click(screen.getByText('Cancel'))
    expect(screen.queryByText('Remove title?')).toBeNull()
  })

  it('shows Building overlay when syncing', () => {
    render(MangaCard, baseProps({ manga: makeManga({ sync_status: 'syncing' }) }))
    expect(screen.getByText('Building…')).toBeInTheDocument()
  })

  it('uses cover_url directly when it starts with http', () => {
    const { container } = render(MangaCard, baseProps({ manga: makeManga({ cover_url: 'https://cdn.example.com/cover.jpg' }) }))
    const img = container.querySelector('img')!
    expect(img.src).toContain('cdn.example.com')
  })

  it('uses /api/ cover_url directly without falling back to coverUrl', () => {
    const { container } = render(MangaCard, baseProps({ manga: makeManga({ cover_url: '/api/media/meta-cover?key=one%20piece' }) }))
    const img = container.querySelector('img')!
    expect(img.src).toContain('/api/media/meta-cover')
    expect(img.src).not.toContain('/covers/')
  })

  it('falls back to api.coverUrl when cover_url is null', () => {
    const { container } = render(MangaCard, baseProps({ manga: makeManga({ cover_url: null }) }))
    const img = container.querySelector('img')!
    expect(img.src).toContain('/covers/m1')
  })

  it('falls back to api.coverUrl on img error', async () => {
    const { container } = render(MangaCard, baseProps({ manga: makeManga({ cover_url: 'https://cdn.example.com/broken.jpg' }) }))
    container.querySelector('img')!.dispatchEvent(new Event('error', { bubbles: true }))
    await vi.waitFor(() => {
      expect(container.querySelector('.text-4xl')).toBeTruthy()
    })
  })

  it('shows 18+ pill when is_explicit is true', () => {
    render(MangaCard, baseProps({ manga: makeManga({ is_explicit: true }) }))
    expect(screen.getByText('18+')).toBeInTheDocument()
  })

  it('does not show 18+ pill when is_explicit is false', () => {
    render(MangaCard, baseProps({ manga: makeManga({ is_explicit: false }) }))
    expect(screen.queryByText('18+')).not.toBeInTheDocument()
  })
})
