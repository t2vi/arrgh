import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import NumberStepper from './NumberStepper.svelte'

describe('NumberStepper', () => {
  it('renders current value', () => {
    render(NumberStepper, { value: 3, min: 1, max: 10, onChange: () => {} })
    expect(screen.getByText('3')).toBeInTheDocument()
  })

  it('calls onChange with incremented value', async () => {
    const onChange = vi.fn()
    render(NumberStepper, { value: 3, min: 1, max: 10, onChange })
    await userEvent.click(screen.getByText('+'))
    expect(onChange).toHaveBeenCalledWith(4)
  })

  it('calls onChange with decremented value', async () => {
    const onChange = vi.fn()
    render(NumberStepper, { value: 3, min: 1, max: 10, onChange })
    await userEvent.click(screen.getByText('−'))
    expect(onChange).toHaveBeenCalledWith(2)
  })

  it('disables decrement at min', () => {
    render(NumberStepper, { value: 1, min: 1, max: 10, onChange: () => {} })
    expect(screen.getByText('−')).toBeDisabled()
  })

  it('disables increment at max', () => {
    render(NumberStepper, { value: 10, min: 1, max: 10, onChange: () => {} })
    expect(screen.getByText('+')).toBeDisabled()
  })
})
