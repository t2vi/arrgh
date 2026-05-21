import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { vi } from 'vitest'
import { SourcesSection } from './SourcesSection'
import { api } from '@/api'
import type { SourceRow, PluginIndexEntry } from '@/api'

vi.mock('@/api', () => ({
  api: {
    listSources:     vi.fn(),
    listPluginIndex: vi.fn(),
    installPlugin:   vi.fn(),
    addSource:       vi.fn(),
    patchSource:     vi.fn(),
    deleteSource:    vi.fn(),
  },
}))

function makeSource(overrides: Partial<SourceRow> = {}): SourceRow {
  return {
    id: 'src-1',
    name: 'MangaDex',
    base_url: 'http://localhost:4000/mangadex',
    enabled: true,
    has_api_key: false,
    is_community: false,
    content_types: ['manga'],
    ...overrides,
  }
}

function makePlugin(overrides: Partial<PluginIndexEntry> = {}): PluginIndexEntry {
  return {
    id: 'myplugin',
    name: 'My Plugin',
    version: '1.0.0',
    description: 'A test plugin',
    download_url: 'http://example.com/myplugin.js',
    bundled: false,
    default_explicit: false,
    content_types: ['manga'],
    ...overrides,
  }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.listSources).mockResolvedValue([])
  vi.mocked(api.listPluginIndex).mockResolvedValue([])
  vi.mocked(api.installPlugin).mockResolvedValue(undefined)
  vi.mocked(api.addSource).mockResolvedValue(makeSource())
  vi.mocked(api.patchSource).mockResolvedValue(undefined)
  vi.mocked(api.deleteSource).mockResolvedValue(undefined)
})

describe('SourcesSection', () => {
  it('opens browse modal when Browse plugins button clicked', async () => {
    render(<SourcesSection />)
    await waitFor(() => expect(screen.getByText('No external sources yet.')).toBeInTheDocument())

    await userEvent.click(screen.getByText('Browse plugins'))
    expect(screen.getByText('Browse Plugins')).toBeInTheDocument()
  })

  it('closes browse modal when X button clicked', async () => {
    const { container } = render(<SourcesSection />)
    await waitFor(() => expect(screen.getByText('No external sources yet.')).toBeInTheDocument())

    await userEvent.click(screen.getByText('Browse plugins'))
    expect(screen.getByText('Browse Plugins')).toBeInTheDocument()

    // The X button is the first (and only unlabeled) button inside the modal overlay
    const closeBtn = container.querySelector('.fixed button') as HTMLButtonElement
    await userEvent.click(closeBtn)
    expect(screen.queryByText('Browse Plugins')).toBeNull()
  })

  it('install plugin calls api.installPlugin and reloads sources', async () => {
    vi.mocked(api.listPluginIndex).mockResolvedValue([makePlugin({ id: 'myplugin' })])
    render(<SourcesSection />)
    await waitFor(() => expect(screen.getByText('No external sources yet.')).toBeInTheDocument())

    await userEvent.click(screen.getByText('Browse plugins'))
    await waitFor(() => expect(screen.getByText('My Plugin')).toBeInTheDocument())

    vi.mocked(api.listSources).mockClear()
    await userEvent.click(screen.getByText('Install'))
    expect(api.installPlugin).toHaveBeenCalledWith('myplugin')
    await waitFor(() => expect(api.listSources).toHaveBeenCalledTimes(1))
  })

  it('shows error when add source fails with 502', async () => {
    vi.mocked(api.addSource).mockRejectedValue(new Error('502'))
    render(<SourcesSection />)
    await waitFor(() => expect(screen.getByText('No external sources yet.')).toBeInTheDocument())

    await userEvent.type(screen.getByPlaceholderText('http://localhost:4000'), 'http://bad-url')
    await userEvent.click(screen.getByText('Add'))

    await waitFor(() =>
      expect(screen.getByText('Could not reach plugin — check the URL.')).toBeInTheDocument()
    )
  })

  it('toggle source calls patchSource with flipped enabled', async () => {
    vi.mocked(api.listSources).mockResolvedValue([makeSource({ id: 'src-1', enabled: true })])
    render(<SourcesSection />)
    await waitFor(() => expect(screen.getByText('MangaDex')).toBeInTheDocument())

    await userEvent.click(screen.getByTitle('Enabled — click to disable'))
    expect(api.patchSource).toHaveBeenCalledWith('src-1', false)
  })
})
