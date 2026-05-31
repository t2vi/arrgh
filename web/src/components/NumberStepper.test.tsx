import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { NumberStepper } from './NumberStepper'
beforeEach(async () => {
  await allure.epic('Components')
  await allure.feature('UI Controls')
})

describe('NumberStepper', () => {
  it('renders current value', () => {
    render(<NumberStepper value={3} min={1} max={10} onChange={() => {}} />)
    expect(screen.getByText('3')).toBeInTheDocument()
  })

  it('calls onChange with incremented value', async () => {
    const onChange = vi.fn()
    render(<NumberStepper value={3} min={1} max={10} onChange={onChange} />)
    await userEvent.click(screen.getByText('+'))
    expect(onChange).toHaveBeenCalledWith(4)
  })

  it('calls onChange with decremented value', async () => {
    const onChange = vi.fn()
    render(<NumberStepper value={3} min={1} max={10} onChange={onChange} />)
    await userEvent.click(screen.getByText('−'))
    expect(onChange).toHaveBeenCalledWith(2)
  })

  it('disables decrement at min', () => {
    render(<NumberStepper value={1} min={1} max={10} onChange={() => {}} />)
    expect(screen.getByText('−')).toBeDisabled()
  })

  it('disables increment at max', () => {
    render(<NumberStepper value={10} min={1} max={10} onChange={() => {}} />)
    expect(screen.getByText('+')).toBeDisabled()
  })
})
