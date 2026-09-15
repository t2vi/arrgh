import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import SegmentedControl from './SegmentedControl.svelte'

const OPTIONS = [
  { value: 'paged', label: 'Paged' },
  { value: 'scroll', label: 'Scroll' },
]

describe('SegmentedControl', () => {
  it('renders all options', () => {
    render(SegmentedControl, { value: 'paged', options: OPTIONS, onChange: () => {} })
    expect(screen.getByText('Paged')).toBeInTheDocument()
    expect(screen.getByText('Scroll')).toBeInTheDocument()
  })

  it('calls onChange with selected value', async () => {
    const onChange = vi.fn()
    render(SegmentedControl, { value: 'paged', options: OPTIONS, onChange })
    await userEvent.click(screen.getByText('Scroll'))
    expect(onChange).toHaveBeenCalledWith('scroll')
  })

  it('does not call onChange when clicking active option', async () => {
    const onChange = vi.fn()
    render(SegmentedControl, { value: 'paged', options: OPTIONS, onChange })
    await userEvent.click(screen.getByText('Paged'))
    expect(onChange).toHaveBeenCalledWith('paged')
  })
})
