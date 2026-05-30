import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ContentTypeFilter } from './ContentTypeFilter'

describe('ContentTypeFilter', () => {
  it('returns null when availableTypes is empty', () => {
    const { container } = render(
      <ContentTypeFilter value={undefined} onChange={() => {}} availableTypes={new Set()} />
    )
    expect(container.firstChild).toBeNull()
  })

  it('shows "All" and only types in availableTypes', () => {
    render(
      <ContentTypeFilter
        value={undefined}
        onChange={() => {}}
        availableTypes={new Set(['manga', 'manhwa'])}
      />
    )
    expect(screen.getByText('All')).toBeInTheDocument()
    expect(screen.getByText('Manga')).toBeInTheDocument()
    expect(screen.getByText('Manhwa')).toBeInTheDocument()
    expect(screen.queryByText('Manhua')).toBeNull()
    expect(screen.queryByText('One-shot')).toBeNull()
  })

  it('calls onChange(undefined) when "All" is clicked', async () => {
    const onChange = vi.fn()
    render(
      <ContentTypeFilter value="manga" onChange={onChange} availableTypes={new Set(['manga'])} />
    )
    await userEvent.click(screen.getByText('All'))
    expect(onChange).toHaveBeenCalledWith(undefined)
  })

  it('calls onChange with the content type value when a pill is clicked', async () => {
    const onChange = vi.fn()
    render(
      <ContentTypeFilter value={undefined} onChange={onChange} availableTypes={new Set(['manga', 'manhwa'])} />
    )
    await userEvent.click(screen.getByText('Manhwa'))
    expect(onChange).toHaveBeenCalledWith('manhwa')
  })

  // hentai appears in fan-out results when allow_explicit=true
  it('shows Hentai pill when hentai is in availableTypes', () => {
    render(
      <ContentTypeFilter value={undefined} onChange={() => {}} availableTypes={new Set(['manga', 'hentai'])} />
    )
    expect(screen.getByText('Hentai')).toBeInTheDocument()
  })

  it('calls onChange("hentai") when Hentai pill is clicked', async () => {
    const onChange = vi.fn()
    render(
      <ContentTypeFilter value={undefined} onChange={onChange} availableTypes={new Set(['manga', 'hentai'])} />
    )
    await userEvent.click(screen.getByText('Hentai'))
    expect(onChange).toHaveBeenCalledWith('hentai')
  })

  it('does not show Hentai pill when hentai not in availableTypes', () => {
    render(
      <ContentTypeFilter value={undefined} onChange={() => {}} availableTypes={new Set(['manga', 'manhwa'])} />
    )
    expect(screen.queryByText('Hentai')).toBeNull()
  })

  it('shows Novel pill when novel is in availableTypes', () => {
    render(
      <ContentTypeFilter value={undefined} onChange={() => {}} availableTypes={new Set(['manga', 'novel'])} />
    )
    expect(screen.getByText('Novel')).toBeInTheDocument()
  })
})
