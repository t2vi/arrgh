import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SegmentedControl } from './SegmentedControl'

const OPTIONS = [
  { value: 'paged', label: 'Paged' },
  { value: 'scroll', label: 'Scroll' },
]

describe('SegmentedControl', () => {
  it('renders all options', () => {
    render(<SegmentedControl value="paged" options={OPTIONS} onChange={() => {}} />)
    expect(screen.getByText('Paged')).toBeInTheDocument()
    expect(screen.getByText('Scroll')).toBeInTheDocument()
  })

  it('calls onChange with selected value', async () => {
    const onChange = vi.fn()
    render(<SegmentedControl value="paged" options={OPTIONS} onChange={onChange} />)
    await userEvent.click(screen.getByText('Scroll'))
    expect(onChange).toHaveBeenCalledWith('scroll')
  })

  it('does not call onChange when clicking active option', async () => {
    const onChange = vi.fn()
    render(<SegmentedControl value="paged" options={OPTIONS} onChange={onChange} />)
    await userEvent.click(screen.getByText('Paged'))
    expect(onChange).toHaveBeenCalledWith('paged')
  })
})
