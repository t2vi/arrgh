import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ZoomControl from './ZoomControl.svelte'
import { createImageZoom } from '../../lib/readerPrefs.svelte'

const STORAGE_KEY = 'reader-image-zoom'

beforeEach(() => {
  localStorage.clear()
})

describe('createImageZoom', () => {
  it('defaults to 100 when no stored value', () => {
    const pref = createImageZoom()
    expect(pref.zoom).toBe(100)
  })

  it('reads stored value from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '150')
    const pref = createImageZoom()
    expect(pref.zoom).toBe(150)
  })

  it('falls back to 100 for invalid stored value', () => {
    localStorage.setItem(STORAGE_KEY, '999')
    const pref = createImageZoom()
    expect(pref.zoom).toBe(100)
  })

  it('apply updates state and persists to localStorage', () => {
    const pref = createImageZoom()
    pref.apply(125)
    expect(pref.zoom).toBe(125)
    expect(localStorage.getItem(STORAGE_KEY)).toBe('125')
  })
})

describe('ZoomControl', () => {
  it('renders zoom button', () => {
    render(ZoomControl, { zoom: 100, onApply: vi.fn() })
    expect(screen.getByTitle('Zoom')).toBeInTheDocument()
  })

  it('popover hidden initially', () => {
    render(ZoomControl, { zoom: 100, onApply: vi.fn() })
    expect(screen.queryByText('50%')).not.toBeInTheDocument()
  })

  it('opens popover on click showing all levels', async () => {
    render(ZoomControl, { zoom: 100, onApply: vi.fn() })
    await userEvent.click(screen.getByTitle('Zoom'))
    expect(screen.getByText('50%')).toBeInTheDocument()
    expect(screen.getByText('75%')).toBeInTheDocument()
    expect(screen.getByText('100%')).toBeInTheDocument()
    expect(screen.getByText('125%')).toBeInTheDocument()
    expect(screen.getByText('150%')).toBeInTheDocument()
  })

  it('calls onApply with selected level and closes popover', async () => {
    const onApply = vi.fn()
    render(ZoomControl, { zoom: 100, onApply })
    await userEvent.click(screen.getByTitle('Zoom'))
    await userEvent.click(screen.getByText('150%'))
    expect(onApply).toHaveBeenCalledWith(150)
    expect(screen.queryByText('50%')).not.toBeInTheDocument()
  })
})
