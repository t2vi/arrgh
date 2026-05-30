import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MangaCard } from './MangaCard'
import type { Title } from '@/types'

vi.mock('@/api', () => ({
  api: { coverUrl: (id: string) => `/covers/${id}` },
}))

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

describe('MangaCard', () => {
  it('renders title and author', () => {
    render(<MangaCard manga={makeManga()} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
    expect(screen.getByText('Vinland Saga')).toBeInTheDocument()
    expect(screen.getByText('Makoto Yukimura')).toBeInTheDocument()
  })

  it('shows confirm dialog after trash button click', async () => {
    render(<MangaCard manga={makeManga()} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
    const trashBtn = screen.getByRole('button')
    await userEvent.click(trashBtn)
    expect(screen.getByText('Remove title?')).toBeInTheDocument()
  })

  it('calls onRemove(false) from "Library only" confirm', async () => {
    const onRemove = vi.fn()
    render(<MangaCard manga={makeManga()} onClick={() => {}} onRemove={onRemove} isRemoving={false} />)
    await userEvent.click(screen.getByRole('button'))
    await userEvent.click(screen.getByText('Library only'))
    expect(onRemove).toHaveBeenCalledWith(false)
  })

  it('calls onRemove(true) from "Remove + delete files" confirm', async () => {
    const onRemove = vi.fn()
    render(<MangaCard manga={makeManga()} onClick={() => {}} onRemove={onRemove} isRemoving={false} />)
    await userEvent.click(screen.getByRole('button'))
    await userEvent.click(screen.getByText('Remove + delete files'))
    expect(onRemove).toHaveBeenCalledWith(true)
  })

  it('cancel closes the confirm dialog', async () => {
    render(<MangaCard manga={makeManga()} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
    await userEvent.click(screen.getByRole('button'))
    await userEvent.click(screen.getByText('Cancel'))
    expect(screen.queryByText('Remove title?')).toBeNull()
  })

  it('shows Building overlay when syncing', () => {
    render(<MangaCard manga={makeManga({ sync_status: 'syncing' })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
    expect(screen.getByText('Building…')).toBeInTheDocument()
  })

  it('uses cover_url directly when it starts with http', () => {
    const { container } = render(
      <MangaCard manga={makeManga({ cover_url: 'https://cdn.example.com/cover.jpg' })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />
    )
    const img = container.querySelector('img')!
    expect(img.src).toContain('cdn.example.com')
  })

  it('uses /api/ cover_url directly without falling back to coverUrl', () => {
    const { container } = render(
      <MangaCard manga={makeManga({ cover_url: '/api/media/meta-cover?key=one%20piece' })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />
    )
    const img = container.querySelector('img')!
    expect(img.src).toContain('/api/media/meta-cover')
    expect(img.src).not.toContain('/covers/')
  })

  it('falls back to api.coverUrl when cover_url is null', () => {
    const { container } = render(
      <MangaCard manga={makeManga({ cover_url: null })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />
    )
    const img = container.querySelector('img')!
    expect(img.src).toContain('/covers/m1')
  })

  it('falls back to api.coverUrl on img error', async () => {
    const { container } = render(
      <MangaCard manga={makeManga({ cover_url: 'https://cdn.example.com/broken.jpg' })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />
    )
    container.querySelector('img')!.dispatchEvent(new Event('error', { bubbles: true }))
    await vi.waitFor(() => {
      expect(container.querySelector('.text-4xl')).toBeTruthy()
    })
  })

  describe('amber warning badge', () => {
    it('shows badge when has_sync_warnings=true and total_chapters=0', () => {
      render(<MangaCard manga={makeManga({ has_sync_warnings: true, total_chapters: 0 })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
      expect(screen.getByTitle('Some sources could not be matched — click title to refresh metadata')).toBeInTheDocument()
    })

    it('hides badge when has_sync_warnings=true but total_chapters>0', () => {
      render(<MangaCard manga={makeManga({ has_sync_warnings: true, total_chapters: 5 })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
      expect(screen.queryByTitle('Some sources could not be matched — click title to refresh metadata')).toBeNull()
    })

    it('hides badge when has_sync_warnings=false even with total_chapters=0', () => {
      render(<MangaCard manga={makeManga({ has_sync_warnings: false, total_chapters: 0 })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
      expect(screen.queryByTitle('Some sources could not be matched — click title to refresh metadata')).toBeNull()
    })
  })
})
