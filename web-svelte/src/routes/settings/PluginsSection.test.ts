import { render, screen, waitFor } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../../lib/api', () => ({
  api: {
    listPlugins: vi.fn(),
    updatePlugin: vi.fn(),
    revertPlugin: vi.fn(),
  },
}))

import PluginsSection from './PluginsSection.svelte'
import { api } from '../../lib/api'
import type { PluginStatus, PluginStatusList } from '../../lib/api'

function row(overrides: Partial<PluginStatus> = {}): PluginStatus {
  return {
    id: 'mangadex', name: 'MangaDex', loaded_version: '1.0.0', origin: 'bundled', has_bundled: true,
    catalog_version: '1.0.0', update_available: false, blocked_reason: null,
    ...overrides,
  }
}

function list(plugins: PluginStatus[], catalog: PluginStatusList['catalog'] = 'live'): PluginStatusList {
  return { catalog, host_protocol: 1, plugins }
}

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.updatePlugin).mockResolvedValue(undefined)
  vi.mocked(api.revertPlugin).mockResolvedValue(undefined)
})

describe('PluginsSection', () => {
  // spec: 031/FR-003, 031/FR-010
  it('shows loaded version and origin; unknown when the plugin reports none', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([
      row(),
      row({ id: 'old', name: 'Old Build', loaded_version: null, catalog_version: null }),
    ]))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByText('MangaDex')).toBeInTheDocument())
    expect(screen.getByText('v1.0.0')).toBeInTheDocument()
    expect(screen.getAllByText('bundled').length).toBeGreaterThan(0)
    expect(screen.getByText('version unknown')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /update/i })).not.toBeInTheDocument()
  })

  // spec: 031/FR-003
  it('offers Update when the catalog is newer, and updates on click', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([row({ catalog_version: '1.1.0', update_available: true })]))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByText('1.0.0 → 1.1.0')).toBeInTheDocument())
    await userEvent.click(screen.getByRole('button', { name: 'Update MangaDex' }))
    expect(api.updatePlugin).toHaveBeenCalledWith('mangadex')
    await waitFor(() => expect(api.listPlugins).toHaveBeenCalledTimes(2))
  })

  // spec: 031/FR-006
  it('disables Update and shows the reason when blocked', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([row({
      catalog_version: '2.0.0', update_available: true,
      blocked_reason: 'needs a newer *ARRgh (plugin protocol 2, this one supports 1)',
    })]))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Update MangaDex' })).toBeDisabled())
    expect(screen.getByText(/needs a newer \*ARRgh/)).toBeInTheDocument()
  })

  // spec: 031/FR-012
  it('shows why an update failed', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([row({ catalog_version: '1.1.0', update_available: true })]))
    vi.mocked(api.updatePlugin).mockRejectedValue(new Error('422 checksum mismatch: expected a, got b'))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByRole('button', { name: 'Update MangaDex' })).toBeInTheDocument())
    await userEvent.click(screen.getByRole('button', { name: 'Update MangaDex' }))
    await waitFor(() => expect(screen.getByText(/checksum mismatch/)).toBeInTheDocument())
  })

  // spec: 031/FR-009
  it('offers Revert only for a download over a bundled version', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([
      row({ loaded_version: '1.1.0', origin: 'downloaded' }),
      row({ id: 'community', name: 'Community', origin: 'downloaded', has_bundled: false }),
    ]))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByText('MangaDex')).toBeInTheDocument())
    expect(screen.getAllByRole('button', { name: /revert/i })).toHaveLength(1)
    await userEvent.click(screen.getByRole('button', { name: 'Revert MangaDex' }))
    expect(api.revertPlugin).toHaveBeenCalledWith('mangadex')
  })

  // spec: 031/FR-002
  it('says when the catalog is the fallback copy or unavailable', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([row()], 'fallback'))
    const { unmount } = render(PluginsSection)
    await waitFor(() => expect(screen.getByText(/copy shipped with this version/)).toBeInTheDocument())
    unmount()

    vi.mocked(api.listPlugins).mockResolvedValue(list([row()], null))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByText(/catalog is unavailable/)).toBeInTheDocument())
  })

  it('hides catalog-only plugins that are not loaded', async () => {
    vi.mocked(api.listPlugins).mockResolvedValue(list([
      row(),
      row({ id: 'extra', name: 'Not Loaded', loaded_version: null, origin: null, has_bundled: false }),
    ]))
    render(PluginsSection)
    await waitFor(() => expect(screen.getByText('MangaDex')).toBeInTheDocument())
    expect(screen.queryByText('Not Loaded')).not.toBeInTheDocument()
  })
})
