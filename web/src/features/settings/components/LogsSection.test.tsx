import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { vi } from 'vitest'
import { LogsSection } from './LogsSection'
import { api } from '@/api'
import type { LogEntry } from '@/api'
beforeEach(async () => {
  await allure.epic('Settings')
  await allure.feature('Logs')
})

vi.mock('@/api', () => ({
  api: {
    getLogLevel: vi.fn(),
    getLogs:     vi.fn(),
    setLogLevel: vi.fn(),
  },
}))

function entry(level: LogEntry['level'], message: string): LogEntry {
  return { timestamp: new Date().toISOString(), level, target: 'arrgh_server::test', message }
}

beforeEach(() => {
  vi.mocked(api.getLogLevel).mockResolvedValue({ level: 'INFO' })
  vi.mocked(api.getLogs).mockResolvedValue([])
  vi.mocked(api.setLogLevel).mockResolvedValue(undefined)
})

describe('LogsSection', () => {
  it('renders capture level buttons and show filter', async () => {
    render(<LogsSection />)
    await waitFor(() => expect(screen.queryByText('Loading…')).toBeNull())
    expect(screen.getByText('Capture Level')).toBeInTheDocument()
    for (const l of ['ERROR', 'WARN', 'INFO', 'DEBUG']) {
      expect(screen.getAllByText(l).length).toBeGreaterThan(0)
    }
    expect(screen.getByText('ALL')).toBeInTheDocument()
  })

  it('shows "No entries." when log buffer is empty', async () => {
    render(<LogsSection />)
    await waitFor(() => expect(screen.getByText('No entries.')).toBeInTheDocument())
  })

  it('renders log entries in the table', async () => {
    vi.mocked(api.getLogs).mockResolvedValue([
      entry('INFO',  'server started'),
      entry('WARN',  'plugin probe failed'),
      entry('ERROR', 'db connection lost'),
    ])
    render(<LogsSection />)
    await waitFor(() => expect(screen.getByText('server started')).toBeInTheDocument())
    expect(screen.getByText('plugin probe failed')).toBeInTheDocument()
    expect(screen.getByText('db connection lost')).toBeInTheDocument()
  })

  it('filter hides entries below selected level', async () => {
    vi.mocked(api.getLogs).mockResolvedValue([
      entry('INFO',  'info message'),
      entry('ERROR', 'error message'),
    ])
    render(<LogsSection />)
    await waitFor(() => expect(screen.getByText('info message')).toBeInTheDocument())

    // Click ERROR in the Show filter bar — only ERROR entries should show
    const errorFilterBtns = screen.getAllByRole('button', { name: 'ERROR' })
    // Last button is the one in the Show filter bar
    await userEvent.click(errorFilterBtns[errorFilterBtns.length - 1])

    expect(screen.queryByText('info message')).toBeNull()
    expect(screen.getByText('error message')).toBeInTheDocument()
  })

  it('calls setLogLevel when capture level button clicked', async () => {
    render(<LogsSection />)
    await waitFor(() => expect(screen.queryByText('Loading…')).toBeNull())

    const debugBtns = screen.getAllByText('DEBUG')
    await userEvent.click(debugBtns[0])
    expect(api.setLogLevel).toHaveBeenCalledWith('DEBUG')
  })
})
