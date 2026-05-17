import { render, screen } from '@testing-library/react'
import { SettingRow } from './SettingRow'

describe('SettingRow', () => {
  it('renders label and hint', () => {
    render(<SettingRow label="Workers" hint="1–10"><span>child</span></SettingRow>)
    expect(screen.getByText('Workers')).toBeInTheDocument()
    expect(screen.getByText('1–10')).toBeInTheDocument()
    expect(screen.getByText('child')).toBeInTheDocument()
  })
})
