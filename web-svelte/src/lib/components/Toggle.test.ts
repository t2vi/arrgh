import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import Toggle from './Toggle.svelte'

describe('Toggle', () => {
  it('calls onChange with toggled value', async () => {
    const onChange = vi.fn()
    render(Toggle, { value: false, onChange })
    await userEvent.click(screen.getByRole('switch'))
    expect(onChange).toHaveBeenCalledWith(true)
  })

  it('calls onChange false when currently true', async () => {
    const onChange = vi.fn()
    render(Toggle, { value: true, onChange })
    await userEvent.click(screen.getByRole('switch'))
    expect(onChange).toHaveBeenCalledWith(false)
  })

  it('reflects checked state via aria-checked', async () => {
    const { rerender } = render(Toggle, { value: false, onChange: () => {} })
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'false')
    await rerender({ value: true, onChange: () => {} })
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'true')
  })
})
