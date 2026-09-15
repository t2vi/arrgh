import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FontSizeControl from './FontSizeControl.svelte'
import { createNovelFontSize } from '../../lib/readerPrefs.svelte'

const STORAGE_KEY = 'reader-font-size'

beforeEach(() => {
  localStorage.clear()
})

describe('createNovelFontSize', () => {
  it('defaults to 16 when no stored value', () => {
    const pref = createNovelFontSize()
    expect(pref.size).toBe(16)
  })

  it('reads stored value from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '21')
    const pref = createNovelFontSize()
    expect(pref.size).toBe(21)
  })

  it('falls back to 16 for invalid stored value', () => {
    localStorage.setItem(STORAGE_KEY, '99')
    const pref = createNovelFontSize()
    expect(pref.size).toBe(16)
  })

  it('apply updates state and persists to localStorage', () => {
    const pref = createNovelFontSize()
    pref.apply(18)
    expect(pref.size).toBe(18)
    expect(localStorage.getItem(STORAGE_KEY)).toBe('18')
  })
})

describe('FontSizeControl', () => {
  it('shows Aa button', () => {
    render(FontSizeControl, { size: 16, onApply: vi.fn() })
    expect(screen.getByTitle('Font size')).toBeInTheDocument()
  })

  it('popover hidden initially', () => {
    render(FontSizeControl, { size: 16, onApply: vi.fn() })
    expect(screen.queryByText('14')).not.toBeInTheDocument()
  })

  it('opens popover on Aa click', async () => {
    render(FontSizeControl, { size: 16, onApply: vi.fn() })
    await userEvent.click(screen.getByTitle('Font size'))
    expect(screen.getByText('14')).toBeInTheDocument()
    expect(screen.getByText('18')).toBeInTheDocument()
    expect(screen.getByText('21')).toBeInTheDocument()
  })

  it('calls onApply with selected size and closes popover', async () => {
    const onApply = vi.fn()
    render(FontSizeControl, { size: 16, onApply })
    await userEvent.click(screen.getByTitle('Font size'))
    await userEvent.click(screen.getByText('18'))
    expect(onApply).toHaveBeenCalledWith(18)
    expect(screen.queryByText('14')).not.toBeInTheDocument()
  })
})
