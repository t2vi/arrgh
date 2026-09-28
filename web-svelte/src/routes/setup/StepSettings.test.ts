import { render, screen } from '@testing-library/svelte'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../../lib/api', () => ({
  api: { saveSettings: vi.fn().mockResolvedValue({}) },
}))

import StepSettings from './StepSettings.svelte'
import { api } from '../../lib/api'

beforeEach(() => {
  vi.clearAllMocks()
})

// spec: 014/FR-002
describe('StepSettings — reader mode default', () => {
  it('pre-selects scroll and saves it unchanged when the admin just clicks Save', async () => {
    const onDone = vi.fn()
    render(StepSettings, { onDone })

    await userEvent.click(screen.getByText('Save & go to library'))

    expect(api.saveSettings).toHaveBeenCalledWith(
      expect.objectContaining({ reader_mode: 'scroll' }),
    )
  })
})
