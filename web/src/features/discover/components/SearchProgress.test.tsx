import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { SearchProgress } from './SearchProgress'

beforeEach(() => { vi.useFakeTimers() })
afterEach(() => { vi.useRealTimers() })

describe('SearchProgress', () => {
  describe('searching state (no completedSources)', () => {
    it('shows "Searching sources…" heading', () => {
      render(<SearchProgress />)
      expect(screen.getByText(/searching sources/i)).toBeDefined()
    })

    it('renders skeleton rows', () => {
      const { container } = render(<SearchProgress />)
      // 4 skeleton rows
      const skeletons = container.querySelectorAll('.animate-pulse')
      expect(skeletons.length).toBeGreaterThan(0)
    })

    it('pills appear progressively after timers advance', async () => {
      const { container } = render(<SearchProgress />)
      const allPills = container.querySelectorAll('[data-testid^="source-pill-"]')
      const visibleBefore = Array.from(allPills).filter(
        el => !el.classList.contains('opacity-0')
      ).length
      // Advance past all 6 stagger intervals (120ms × 6 = 720ms) + flush React updates
      await act(async () => { vi.advanceTimersByTime(720) })
      const visibleAfter = Array.from(allPills).filter(
        el => !el.classList.contains('opacity-0')
      ).length
      expect(visibleAfter).toBeGreaterThan(visibleBefore)
    })
  })

  describe('completed state (completedSources provided)', () => {
    it('shows "Results from…" heading', () => {
      render(<SearchProgress completedSources={new Set(['mangaupdates'])} />)
      expect(screen.getByText(/results from/i)).toBeDefined()
    })

    it('does NOT render skeleton rows', () => {
      const { container } = render(<SearchProgress completedSources={new Set(['mangaupdates'])} />)
      // skeleton wrapper only exists during searching
      expect(container.querySelector('[data-testid="skeleton-rows"]')).toBeNull()
    })

    it('all pills are immediately visible (no stagger)', () => {
      const { container } = render(<SearchProgress completedSources={new Set(['mangaupdates'])} />)
      const pills = container.querySelectorAll('[data-testid^="source-pill-"]')
      expect(pills.length).toBe(6)
      const hidden = Array.from(pills).filter(el => el.classList.contains('opacity-0'))
      expect(hidden.length).toBe(0)
    })

    it('matched source pills turn green after stagger', async () => {
      const { container } = render(
        <SearchProgress completedSources={new Set(['mangaupdates', 'anilist'])} />
      )
      // mangaupdates is index 0 — needs settled >= 1, so just one 100ms step
      await act(async () => { vi.advanceTimersByTime(100) })
      const pill = container.querySelector('[data-testid="source-pill-mangaupdates"]')
      expect(pill?.className).toContain('text-green')
    })

    it('unmatched source pills are dimmed after stagger', async () => {
      const { container } = render(
        <SearchProgress completedSources={new Set(['mangaupdates'])} />
      )
      // Flush 100ms at a time so React re-renders between each step, allowing the
      // chained setTimeout stagger to propagate through all 6 sources.
      for (let i = 0; i < 8; i++) {
        await act(async () => { vi.advanceTimersByTime(100) })
      }
      const pill = container.querySelector('[data-testid="source-pill-nhentai"]')
      expect(pill?.className).toContain('opacity-50')
    })

    it('dot on matched source is green, not pulsing after stagger', async () => {
      const { container } = render(
        <SearchProgress completedSources={new Set(['mangaupdates'])} />
      )
      await act(async () => { vi.advanceTimersByTime(100) })
      const pill = container.querySelector('[data-testid="source-pill-mangaupdates"]')
      const dot = pill?.querySelector('[data-testid="source-dot"]')
      expect(dot?.className).toContain('bg-green')
      expect(dot?.className).not.toContain('animate-pulse')
    })
  })
})
