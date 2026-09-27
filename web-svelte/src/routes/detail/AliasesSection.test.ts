// spec: 033/FR-005
import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import AliasesSection from './AliasesSection.svelte'

describe('AliasesSection', () => {
  it('lists current aliases with a remove button each', () => {
    render(AliasesSection, {
      aliases: ['The Primal Hunter'],
      pending: false,
      onAdd: vi.fn(),
      onRemove: vi.fn(),
    })
    expect(screen.getByText('The Primal Hunter')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Remove alias The Primal Hunter' })).toBeTruthy()
  })

  it('adds a trimmed alias and clears the input', async () => {
    const onAdd = vi.fn()
    render(AliasesSection, { aliases: [], pending: false, onAdd, onRemove: vi.fn() })
    const input = screen.getByPlaceholderText('Alternate title a source might use…')
    await userEvent.type(input, '  The Primal Hunter  ')
    await userEvent.click(screen.getByRole('button', { name: 'Add' }))
    expect(onAdd).toHaveBeenCalledWith('The Primal Hunter')
    expect(input).toHaveValue('')
  })

  it('removes an alias', async () => {
    const onRemove = vi.fn()
    render(AliasesSection, { aliases: ['Old Name'], pending: false, onAdd: vi.fn(), onRemove })
    await userEvent.click(screen.getByRole('button', { name: 'Remove alias Old Name' }))
    expect(onRemove).toHaveBeenCalledWith('Old Name')
  })

  it('disables add while a request is pending', () => {
    render(AliasesSection, { aliases: [], pending: true, onAdd: vi.fn(), onRemove: vi.fn() })
    expect(screen.getByRole('button', { name: 'Add' })).toBeDisabled()
  })
})
