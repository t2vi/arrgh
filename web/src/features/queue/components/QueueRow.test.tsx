import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueueRow } from './QueueRow'
import type { QueueItem } from '@/api'
beforeEach(async () => {
  await allure.epic('Queue')
  await allure.feature('Queue')
})

function makeItem(overrides: Partial<QueueItem> = {}): QueueItem {
  return {
    id: '1',
    chapter_id: 'c1',
    manga_title: 'Berserk',
    chapter_num: 42,
    status: 'pending',
    error: null,
    pages_downloaded: 0,
    pages_total: 0,
    created_at: '',
    updated_at: '',
    ...overrides,
  }
}

describe('QueueRow', () => {
  it('renders manga title and chapter number', () => {
    render(<QueueRow item={makeItem()} onRemove={() => {}} />)
    expect(screen.getByText('Berserk')).toBeInTheDocument()
    expect(screen.getByText('Ch. 42')).toBeInTheDocument()
  })

  it('shows remove button for non-downloading items', () => {
    render(<QueueRow item={makeItem({ status: 'pending' })} onRemove={() => {}} />)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })

  it('hides remove button while downloading', () => {
    render(<QueueRow item={makeItem({ status: 'downloading' })} onRemove={() => {}} />)
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('calls onRemove when button is clicked', async () => {
    const onRemove = vi.fn()
    render(<QueueRow item={makeItem({ status: 'done' })} onRemove={onRemove} />)
    await userEvent.click(screen.getByRole('button'))
    expect(onRemove).toHaveBeenCalled()
  })

  it('renders error text when present', () => {
    render(<QueueRow item={makeItem({ status: 'error', error: 'Network timeout' })} onRemove={() => {}} />)
    expect(screen.getByText('Network timeout')).toBeInTheDocument()
  })

  describe('download progress bar', () => {
    it('shows progress bar and percentage when downloading with pages', () => {
      render(<QueueRow item={makeItem({ status: 'downloading', pages_downloaded: 6, pages_total: 20 })} onRemove={() => {}} />)
      expect(screen.getByText('30%')).toBeInTheDocument()
    })

    it('hides progress bar when pages_total is 0', () => {
      render(<QueueRow item={makeItem({ status: 'downloading', pages_downloaded: 0, pages_total: 0 })} onRemove={() => {}} />)
      expect(screen.queryByText(/%/)).toBeNull()
    })

    it('shows 100% when all pages downloaded', () => {
      render(<QueueRow item={makeItem({ status: 'downloading', pages_downloaded: 20, pages_total: 20 })} onRemove={() => {}} />)
      expect(screen.getByText('100%')).toBeInTheDocument()
    })

    it('does not show progress bar when status is pending', () => {
      render(<QueueRow item={makeItem({ status: 'pending', pages_downloaded: 5, pages_total: 20 })} onRemove={() => {}} />)
      expect(screen.queryByText(/%/)).toBeNull()
    })
  })
})
