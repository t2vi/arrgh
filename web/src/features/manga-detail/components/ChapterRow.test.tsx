import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ChapterRow } from './ChapterRow'
import type { Chapter, ReadProgress } from '@/types'
import type { QueueItem } from '@/api'

function makeChapter(overrides: Partial<Chapter> = {}): Chapter {
  return {
    id: 'ch-1',
    manga_id: 'manga-1',
    title: 'The Beginning',
    number: 1,
    volume: null,
    has_sources: true,
    local_path: null,
    page_count: 20,
    downloaded: false,
    chapter_format: 'pages',
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

function makeProgress(overrides: Partial<ReadProgress> = {}): ReadProgress {
  return {
    id: 'p-1',
    chapter_id: 'ch-1',
    current_page: 0,
    completed: false,
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

function makeQueueItem(overrides: Partial<QueueItem> = {}): QueueItem {
  return {
    id: 'q-1',
    chapter_id: 'ch-1',
    manga_title: 'Test Manga',
    chapter_num: 1,
    status: 'pending',
    error: null,
    pages_downloaded: 0,
    pages_total: 0,
    created_at: '',
    updated_at: '',
    ...overrides,
  }
}

const noop = () => {}

describe('ChapterRow', () => {
  it('shows BookOpen button when downloaded', () => {
    render(<ChapterRow chapter={makeChapter({ downloaded: true })} progress={null} queueItem={null} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(screen.getByTitle('Read')).toBeInTheDocument()
  })

  it('shows Download button when not downloaded and has_sources', () => {
    render(<ChapterRow chapter={makeChapter({ downloaded: false, has_sources: true })} progress={null} queueItem={null} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(screen.getByTitle('Download & read')).toBeInTheDocument()
  })

  it('shows no download button when has_sources is false and not downloaded', () => {
    const { container } = render(<ChapterRow chapter={makeChapter({ downloaded: false, has_sources: false })} progress={null} queueItem={null} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    // No clickable button when chapter has no sources
    expect(container.querySelector('button[title]')).toBeNull()
  })

  it('shows spinner when downloading', () => {
    const { container } = render(<ChapterRow chapter={makeChapter()} progress={null} queueItem={makeQueueItem({ status: 'downloading' })} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(container.querySelector('.animate-spin')).toBeInTheDocument()
  })

  it('shows Queued button when pending', () => {
    render(<ChapterRow chapter={makeChapter()} progress={null} queueItem={makeQueueItem({ status: 'pending' })} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(screen.getByText('Queued')).toBeInTheDocument()
  })

  it('calls onCancelDownload when Queued button clicked', async () => {
    const cancel = vi.fn()
    render(<ChapterRow chapter={makeChapter()} progress={null} queueItem={makeQueueItem({ status: 'pending', id: 'q-99' })} pendingRead={false} onOpen={noop} onCancelDownload={cancel} />)
    await userEvent.click(screen.getByText('Queued'))
    expect(cancel).toHaveBeenCalledWith('q-99')
  })

  it('shows AlertCircle when error', () => {
    render(<ChapterRow chapter={makeChapter()} progress={null} queueItem={makeQueueItem({ status: 'error' })} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(screen.getByTitle('Retry')).toBeInTheDocument()
  })

  it('shows download progress bar and % when downloading with pages', () => {
    render(<ChapterRow chapter={makeChapter()} progress={null} queueItem={makeQueueItem({ status: 'downloading', pages_downloaded: 8, pages_total: 20 })} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(screen.getByText('40%')).toBeInTheDocument()
  })

  it('hides download progress bar when pages_total is 0', () => {
    render(<ChapterRow chapter={makeChapter()} progress={null} queueItem={makeQueueItem({ status: 'downloading', pages_downloaded: 0, pages_total: 0 })} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(screen.queryByText(/%/)).toBeNull()
  })

  it('completed chapter has reduced opacity', () => {
    const { container } = render(<ChapterRow chapter={makeChapter({ downloaded: true })} progress={makeProgress({ completed: true })} queueItem={null} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    expect(container.firstChild).toHaveClass('opacity-50')
  })

  it('shows read progress bar when started but not completed', () => {
    const { container } = render(<ChapterRow chapter={makeChapter({ downloaded: true, page_count: 20 })} progress={makeProgress({ current_page: 10, completed: false })} queueItem={null} pendingRead={false} onOpen={noop} onCancelDownload={noop} />)
    const bar = container.querySelector('.bg-primary') as HTMLElement
    expect(bar).toBeInTheDocument()
    expect(bar.style.width).toBe('50%')
  })
})
