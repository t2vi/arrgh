import { render, screen } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import LibraryCoverCard from './LibraryCoverCard.svelte'
import type { Title } from '../../lib/types'

function makeManga(overrides: Partial<Title> = {}): Title {
  return {
    id: 'm1', title: 'Berserk', author: 'Miura', description: null,
    cover_url: null, status: 'ongoing', content_type: 'manga',
    sync_status: 'ready', is_local: false, local_path: null,
    year: null, tags: null, auto_download: null, reader_mode: null,
    download_dir: null, is_explicit: false, has_sync_warnings: false,
    created_at: '', updated_at: '',
    ...overrides,
  }
}

describe('LibraryCoverCard', () => {
  it('shows 18+ pill when is_explicit is true', () => {
    render(LibraryCoverCard, { manga: makeManga({ is_explicit: true }), onClick: () => {} })
    expect(screen.getByText('18+')).toBeInTheDocument()
  })

  it('does not show 18+ pill when is_explicit is false', () => {
    render(LibraryCoverCard, { manga: makeManga({ is_explicit: false }), onClick: () => {} })
    expect(screen.queryByText('18+')).not.toBeInTheDocument()
  })
})
