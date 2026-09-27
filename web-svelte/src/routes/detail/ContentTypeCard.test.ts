// spec: 032/FR-004 — admin can set any content type, hentai included (GH #211)
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import ContentTypeCard from './ContentTypeCard.svelte'
import { api } from '../../lib/api'

vi.mock('../../lib/api', () => ({ api: { setTitleContentType: vi.fn().mockResolvedValue(undefined) } }))

describe('ContentTypeCard', () => {
  it('offers every content type, hentai included', () => {
    render(ContentTypeCard, { mangaId: 't1', value: 'manga' })
    for (const ct of ['manga', 'manhwa', 'manhua', 'novel', 'hentai']) {
      expect(screen.getByRole('button', { name: ct })).toBeTruthy()
    }
  })

  it('saves the picked type', async () => {
    render(ContentTypeCard, { mangaId: 't1', value: 'manga' })
    await userEvent.click(screen.getByRole('button', { name: 'hentai' }))
    expect(api.setTitleContentType).toHaveBeenCalledWith('t1', 'hentai')
  })
})
