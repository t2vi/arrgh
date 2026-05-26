import { renderHook, act } from '@testing-library/react'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import { useNovelFontSize, FontSizeControl } from './FontSizeControl'

const STORAGE_KEY = 'reader-font-size'

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

describe('useNovelFontSize', () => {
  it('defaults to 16 when no stored value', () => {
    const { result } = renderHook(() => useNovelFontSize())
    expect(result.current.size).toBe(16)
  })

  it('reads stored value from localStorage', () => {
    localStorage.setItem(STORAGE_KEY, '21')
    const { result } = renderHook(() => useNovelFontSize())
    expect(result.current.size).toBe(21)
  })

  it('falls back to 16 for invalid stored value', () => {
    localStorage.setItem(STORAGE_KEY, '99')
    const { result } = renderHook(() => useNovelFontSize())
    expect(result.current.size).toBe(16)
  })

  it('apply updates state and persists to localStorage', () => {
    const { result } = renderHook(() => useNovelFontSize())
    act(() => { result.current.apply(18) })
    expect(result.current.size).toBe(18)
    expect(localStorage.getItem(STORAGE_KEY)).toBe('18')
  })
})

describe('FontSizeControl', () => {
  it('shows Aa button', () => {
    render(<FontSizeControl size={16} onApply={vi.fn()} />)
    expect(screen.getByTitle('Font size')).toBeInTheDocument()
  })

  it('popover hidden initially', () => {
    render(<FontSizeControl size={16} onApply={vi.fn()} />)
    expect(screen.queryByText('14')).not.toBeInTheDocument()
  })

  it('opens popover on Aa click', async () => {
    render(<FontSizeControl size={16} onApply={vi.fn()} />)
    await userEvent.click(screen.getByTitle('Font size'))
    expect(screen.getByText('14')).toBeInTheDocument()
    expect(screen.getByText('18')).toBeInTheDocument()
    expect(screen.getByText('21')).toBeInTheDocument()
  })

  it('calls onApply with selected size and closes popover', async () => {
    const onApply = vi.fn()
    render(<FontSizeControl size={16} onApply={onApply} />)
    await userEvent.click(screen.getByTitle('Font size'))
    await userEvent.click(screen.getByText('18'))
    expect(onApply).toHaveBeenCalledWith(18)
    expect(screen.queryByText('14')).not.toBeInTheDocument()
  })
})
