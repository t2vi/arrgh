import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MangaCard } from './MangaCard'
import type { Manga } from '@/types'

vi.mock('@/api', () => ({
  api: { coverUrl: (id: string) => `/covers/${id}` },
}))

function makeManga(overrides: Partial<Manga> = {}): Manga {
  return {
    id: 'm1',
    title: 'Vinland Saga',
    author: 'Makoto Yukimura',
    description: null,
    cover_url: null,
    status: 'ongoing',
    content_type: 'manga',
    sync_status: 'ready',
    source: 'mangadex',
    source_id: 'src1',
    local_path: null,
    year: null,
    tags: null,
    auto_download: null,
    reader_mode: null,
    download_dir: null,
    is_explicit: false,
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
    expect(screen.getByText('Remove manga?')).toBeInTheDocument()
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
    expect(screen.queryByText('Remove manga?')).toBeNull()
  })

  it('shows Building overlay when syncing', () => {
    render(<MangaCard manga={makeManga({ sync_status: 'syncing' })} onClick={() => {}} onRemove={() => {}} isRemoving={false} />)
    expect(screen.getByText('Building…')).toBeInTheDocument()
  })
})
