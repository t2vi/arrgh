import { render, screen } from '@testing-library/svelte'
import { createRawSnippet } from 'svelte'
import { describe, expect, it } from 'vitest'
import SettingRow from './SettingRow.svelte'

describe('SettingRow', () => {
  it('renders label and hint', () => {
    const children = createRawSnippet(() => ({
      render: () => '<span>child</span>',
    }))
    render(SettingRow, { label: 'Workers', hint: '1–10', children })
    expect(screen.getByText('Workers')).toBeInTheDocument()
    expect(screen.getByText('1–10')).toBeInTheDocument()
    expect(screen.getByText('child')).toBeInTheDocument()
  })
})
