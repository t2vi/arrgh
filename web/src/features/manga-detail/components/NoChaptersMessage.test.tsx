import { render, screen } from '@testing-library/react'
import { vi, describe, it, expect } from 'vitest'
import { NoChaptersMessage } from './NoChaptersMessage'
beforeEach(async () => {
  await allure.epic('Library')
  await allure.feature('Title Detail')
})

describe('NoChaptersMessage', () => {
  it('shows "no sources" message when hasSyncWarnings is true', () => {
    render(
      <NoChaptersMessage
        hasSyncWarnings={true}
        isSyncing={false}
        isPending={false}
        isRemoteSource={true}
        onSync={() => {}}
      />
    )
    expect(screen.getByText(/no sources found/i)).toBeTruthy()
  })

  it('does not show "no sources" message when hasSyncWarnings is false', () => {
    render(
      <NoChaptersMessage
        hasSyncWarnings={false}
        isSyncing={false}
        isPending={false}
        isRemoteSource={true}
        onSync={() => {}}
      />
    )
    expect(screen.queryByText(/no sources found/i)).toBeNull()
    expect(screen.getByText(/no chapters/i)).toBeTruthy()
  })

  it('shows syncing spinner when isSyncing', () => {
    render(
      <NoChaptersMessage
        hasSyncWarnings={false}
        isSyncing={true}
        isPending={false}
        isRemoteSource={true}
        onSync={() => {}}
      />
    )
    expect(screen.getByText(/fetching chapters/i)).toBeTruthy()
  })

  it('shows syncing spinner when isPending', () => {
    render(
      <NoChaptersMessage
        hasSyncWarnings={false}
        isSyncing={false}
        isPending={true}
        isRemoteSource={true}
        onSync={() => {}}
      />
    )
    expect(screen.getByText(/fetching chapters/i)).toBeTruthy()
  })

  it('no Retry button when hasSyncWarnings=true but isRemoteSource=false', () => {
    render(
      <NoChaptersMessage
        hasSyncWarnings={true}
        isSyncing={false}
        isPending={false}
        isRemoteSource={false}
        onSync={() => {}}
      />
    )
    expect(screen.getByText(/no sources found/i)).toBeTruthy()
    expect(screen.queryByText(/retry/i)).toBeNull()
  })
})
