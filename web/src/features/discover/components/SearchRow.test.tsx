import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SearchRow } from './SearchRow'
import type { SearchResult } from '@/api'
beforeEach(async () => {
  await allure.epic('Discover')
  await allure.feature('Search')
})

vi.mock('@/api', () => ({
  api: { proxyImageUrl: (url: string) => `/proxy?url=${url}` },
}))

function makeResult(overrides: Partial<SearchResult> = {}): SearchResult {
  return {
    mangaupdates_id: 'mu-001',
    title: 'Solo Leveling',
    description: 'A hunter awakens.',
    cover_url: null,
    status: 'ongoing',
    author: 'Chugong',
    year: 2018,
    tags: null,
    content_type: 'manhwa',
    in_library: false,
    library_id: null,
    source: 'mangaupdates',
    is_explicit: false,
    ...overrides,
  }
}

function baseProps(overrides: Partial<Parameters<typeof SearchRow>[0]> = {}) {
  return {
    result: makeResult(),
    inLibrary: false,
    addingId: null,
    libraryId: undefined,
    onAdd: vi.fn(),
    onView: vi.fn(),
    ...overrides,
  }
}

describe('SearchRow', () => {
  it('renders title and author', () => {
    render(<SearchRow {...baseProps()} />)
    expect(screen.getByText('Solo Leveling')).toBeInTheDocument()
    expect(screen.getByText('Chugong')).toBeInTheDocument()
  })

  it('renders description when present', () => {
    render(<SearchRow {...baseProps()} />)
    expect(screen.getByText('A hunter awakens.')).toBeInTheDocument()
  })

  // is_explicit drives 18+ badge — not tag inference
  it('shows 18+ badge when is_explicit is true', () => {
    render(<SearchRow {...baseProps({ result: makeResult({ is_explicit: true }) })} />)
    expect(screen.getByText('18+')).toBeInTheDocument()
  })

  it('does not show 18+ badge when is_explicit is false', () => {
    render(<SearchRow {...baseProps({ result: makeResult({ is_explicit: false }) })} />)
    expect(screen.queryByText('18+')).not.toBeInTheDocument()
  })

  // Server sets is_explicit; client must NOT derive it from tags
  it('does not show 18+ badge for adult tag when is_explicit is false', () => {
    render(<SearchRow {...baseProps({ result: makeResult({ is_explicit: false, tags: 'adult,action' }) })} />)
    expect(screen.queryByText('18+')).not.toBeInTheDocument()
  })

  it('shows 18+ badge for hentai content_type only when is_explicit is true', () => {
    render(<SearchRow {...baseProps({ result: makeResult({ is_explicit: true, content_type: 'hentai' }) })} />)
    expect(screen.getByText('18+')).toBeInTheDocument()
  })

  it('shows Add button when not in library', () => {
    render(<SearchRow {...baseProps({ inLibrary: false })} />)
    expect(screen.getByRole('button', { name: /add/i })).toBeInTheDocument()
  })

  it('calls onAdd when Add button is clicked', async () => {
    const onAdd = vi.fn()
    render(<SearchRow {...baseProps({ onAdd })} />)
    await userEvent.click(screen.getByRole('button', { name: /add/i }))
    expect(onAdd).toHaveBeenCalledOnce()
  })

  it('shows In Library button when inLibrary is true', () => {
    render(<SearchRow {...baseProps({ inLibrary: true, libraryId: 'lib-1' })} />)
    expect(screen.getByRole('button', { name: /in library/i })).toBeInTheDocument()
  })

  it('calls onView with libraryId when In Library button is clicked', async () => {
    const onView = vi.fn()
    render(<SearchRow {...baseProps({ inLibrary: true, libraryId: 'lib-1', onView })} />)
    await userEvent.click(screen.getByRole('button', { name: /in library/i }))
    expect(onView).toHaveBeenCalledWith('lib-1')
  })

  // isAdding is driven by addingId === result.mangaupdates_id
  it('shows loading indicator when addingId matches result id', () => {
    render(<SearchRow {...baseProps({ addingId: 'mu-001' })} />)
    expect(screen.getByText('…')).toBeInTheDocument()
  })

  it('Add button is disabled while adding', () => {
    render(<SearchRow {...baseProps({ addingId: 'mu-001' })} />)
    expect(screen.getByRole('button', { name: '…' })).toBeDisabled()
  })

  it('Add button is enabled when addingId refers to a different result', () => {
    render(<SearchRow {...baseProps({ addingId: 'other-id', result: makeResult({ mangaupdates_id: 'mu-001' }) })} />)
    expect(screen.getByRole('button', { name: /add/i })).not.toBeDisabled()
  })

  it('renders cover image when cover_url is provided', () => {
    const { container } = render(
      <SearchRow {...baseProps({ result: makeResult({ cover_url: 'https://cdn.example.com/cover.jpg' }) })} />
    )
    const img = container.querySelector('img')
    expect(img).toBeTruthy()
    expect(img!.src).toContain('cdn.example.com')
  })

  it('renders cover placeholder (pulse skeleton) when no cover_url', () => {
    const { container } = render(<SearchRow {...baseProps()} />)
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('.animate-pulse')).toBeTruthy()
  })
})
