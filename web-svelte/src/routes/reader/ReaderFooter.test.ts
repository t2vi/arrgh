import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ReaderFooter from './ReaderFooter.svelte'
import { router } from '../../lib/router.svelte'
import type { Chapter } from '../../lib/types'

const stubChapter = (id: string): Chapter => ({
  id,
  title_id: 'title-1',
  title: null,
  number: 1,
  volume: null,
  local_path: null,
  page_count: 10,
  downloaded: true,
  has_sources: true,
  chapter_format: 'image',
  created_at: '2026-01-01T00:00:00Z',
})

const prev = stubChapter('prev-id')
const next = stubChapter('next-id')

beforeEach(() => {
  vi.spyOn(router, 'navigate').mockImplementation(() => {})
})

describe('ReaderFooter — paged mode', () => {
  it('shows Prev/Next page buttons mid-chapter', () => {
    render(ReaderFooter, {
      mode: 'paged', page: 5, total: 10, totalLabel: '10', atEnd: false,
      prevChapter: prev, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    expect(screen.getByText('Prev')).toBeInTheDocument()
    expect(screen.getByText('Next')).toBeInTheDocument()
  })

  it('prev button disabled on page 0, enabled otherwise', () => {
    const { rerender } = render(ReaderFooter, {
      mode: 'paged', page: 0, total: 10, totalLabel: '10', atEnd: false,
      prevChapter: null, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    expect(screen.getByText('Prev').closest('button')).toBeDisabled()

    rerender({
      mode: 'paged', page: 3, total: 10, totalLabel: '10', atEnd: false,
      prevChapter: null, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    expect(screen.getByText('Prev').closest('button')).not.toBeDisabled()
  })

  it('at last page with next chapter — Next becomes "Next Ch." and navigates', async () => {
    render(ReaderFooter, {
      mode: 'paged', page: 9, total: 10, totalLabel: '10', atEnd: true,
      prevChapter: prev, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    const btn = screen.getByText('Next Ch.').closest('button')!
    expect(btn).not.toBeDisabled()
    await userEvent.click(btn)
    expect(router.navigate).toHaveBeenCalledWith('/reader/next-id')
  })

  it('at last page with no next chapter — Next disabled', () => {
    render(ReaderFooter, {
      mode: 'paged', page: 9, total: 10, totalLabel: '10', atEnd: true,
      prevChapter: prev, nextChapter: null,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    expect(screen.getByText('Next').closest('button')).toBeDisabled()
  })

  it('at first page with prev chapter — Prev becomes "Prev Ch." and navigates', async () => {
    render(ReaderFooter, {
      mode: 'paged', page: 0, total: 10, totalLabel: '10', atEnd: false,
      prevChapter: prev, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    const btn = screen.getByText('Prev Ch.').closest('button')!
    expect(btn).not.toBeDisabled()
    await userEvent.click(btn)
    expect(router.navigate).toHaveBeenCalledWith('/reader/prev-id')
  })

  it('mid-chapter next page fires onNextPage', async () => {
    const onNextPage = vi.fn()
    render(ReaderFooter, {
      mode: 'paged', page: 3, total: 10, totalLabel: '10', atEnd: false,
      prevChapter: prev, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage,
    })
    await userEvent.click(screen.getByText('Next').closest('button')!)
    expect(onNextPage).toHaveBeenCalled()
  })
})

describe('ReaderFooter — scroll/novel mode', () => {
  it('shows Prev Ch. / Next Ch. buttons', () => {
    render(ReaderFooter, {
      mode: 'novel', page: 0, total: null, totalLabel: '?', atEnd: false,
      prevChapter: prev, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    expect(screen.getByText('Prev Ch.')).toBeInTheDocument()
    expect(screen.getByText('Next Ch.')).toBeInTheDocument()
  })

  it('both disabled when no adjacent chapters', () => {
    render(ReaderFooter, {
      mode: 'scroll', page: 0, total: null, totalLabel: '?', atEnd: false,
      prevChapter: null, nextChapter: null,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    expect(screen.getByText('Prev Ch.').closest('button')).toBeDisabled()
    expect(screen.getByText('Next Ch.').closest('button')).toBeDisabled()
  })

  it('next chapter navigates on click', async () => {
    render(ReaderFooter, {
      mode: 'novel', page: 0, total: null, totalLabel: '?', atEnd: false,
      prevChapter: null, nextChapter: next,
      onPrevPage: vi.fn(), onNextPage: vi.fn(),
    })
    await userEvent.click(screen.getByText('Next Ch.').closest('button')!)
    expect(router.navigate).toHaveBeenCalledWith('/reader/next-id')
  })
})
