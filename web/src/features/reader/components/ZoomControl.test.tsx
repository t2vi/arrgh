import { renderHook, act } from '@testing-library/react'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import { useImageZoom, ZoomControl } from './ZoomControl'
beforeEach(async () => {
  await allure.epic('Reader')
  await allure.feature('Zoom')
})

const STORAGE_KEY = 'reader-image-zoom'

const localStorageMock = (() => {
  let store: Record<string, string> = {}
  return {
    getItem: (k: string) => store[k] ?? null,
    setItem: (k: string, v: string) => { store[k] = v },
    removeItem: (k: string) => { delete store[k] },
    clear: () => { store = {} },
  }
})()

vi.stubGlobal('localStorage', localStorageMock)

beforeEach(() => {
  localStorageMock.clear()
  vi.clearAllMocks()
})

describe('useImageZoom', () => {
  it('defaults to 100 when no stored value', () => {
    const { result } = renderHook(() => useImageZoom())
    expect(result.current.zoom).toBe(100)
  })

  it('reads stored value from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '150')
    const { result } = renderHook(() => useImageZoom())
    expect(result.current.zoom).toBe(150)
  })

  it('falls back to 100 for invalid stored value', () => {
    localStorage.setItem(STORAGE_KEY, '999')
    const { result } = renderHook(() => useImageZoom())
    expect(result.current.zoom).toBe(100)
  })

  it('apply updates state and persists to localStorage', () => {
    const { result } = renderHook(() => useImageZoom())
    act(() => { result.current.apply(125) })
    expect(result.current.zoom).toBe(125)
    expect(localStorage.getItem(STORAGE_KEY)).toBe('125')
  })
})

describe('ZoomControl', () => {
  it('renders zoom button', () => {
    render(<ZoomControl zoom={100} onApply={vi.fn()} />)
    expect(screen.getByTitle('Zoom')).toBeInTheDocument()
  })

  it('popover hidden initially', () => {
    render(<ZoomControl zoom={100} onApply={vi.fn()} />)
    expect(screen.queryByText('50%')).not.toBeInTheDocument()
  })

  it('opens popover on click showing all levels', async () => {
    render(<ZoomControl zoom={100} onApply={vi.fn()} />)
    await userEvent.click(screen.getByTitle('Zoom'))
    expect(screen.getByText('50%')).toBeInTheDocument()
    expect(screen.getByText('75%')).toBeInTheDocument()
    expect(screen.getByText('100%')).toBeInTheDocument()
    expect(screen.getByText('125%')).toBeInTheDocument()
    expect(screen.getByText('150%')).toBeInTheDocument()
  })

  it('calls onApply with selected level and closes popover', async () => {
    const onApply = vi.fn()
    render(<ZoomControl zoom={100} onApply={onApply} />)
    await userEvent.click(screen.getByTitle('Zoom'))
    await userEvent.click(screen.getByText('150%'))
    expect(onApply).toHaveBeenCalledWith(150)
    expect(screen.queryByText('50%')).not.toBeInTheDocument()
  })
})
