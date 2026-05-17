import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueueRow } from './QueueRow'
import type { QueueItem } from '@/api'

function makeItem(overrides: Partial<QueueItem> = {}): QueueItem {
  return {
    id: '1',
    chapter_id: 'c1',
    manga_title: 'Berserk',
    chapter_num: 42,
    status: 'pending',
    error: null,
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
})
