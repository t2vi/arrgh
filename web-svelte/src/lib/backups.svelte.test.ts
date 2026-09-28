import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./api', () => ({
  api: {
    listBackups: vi.fn(),
    createBackup: vi.fn(),
    deleteBackup: vi.fn(),
    restoreBackup: vi.fn(),
    restoreBackupUpload: vi.fn(),
  },
}))

import { BackupsStore } from './backups.svelte'
import { api } from './api'

const BACKUPS = [
  { filename: 'arrgh-backup-2026-01-02T00-00-00.db', created_at: '2026-01-02T00:00:00', size_bytes: 2048 },
  { filename: 'arrgh-backup-2026-01-01T00-00-00.db', created_at: '2026-01-01T00:00:00', size_bytes: 1024 },
]

beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.listBackups).mockResolvedValue(BACKUPS)
  vi.mocked(api.createBackup).mockResolvedValue(undefined)
  vi.mocked(api.deleteBackup).mockResolvedValue(undefined)
  vi.mocked(api.restoreBackup).mockResolvedValue(undefined)
  vi.mocked(api.restoreBackupUpload).mockResolvedValue(undefined)
})

describe('BackupsStore', () => {
  it('loads backups on construction', async () => {
    const store = new BackupsStore()
    await vi.waitFor(() => expect(store.backups).toEqual(BACKUPS))
    expect(store.loading).toBe(false)
  })

  it('backupNow creates then reloads', async () => {
    const store = new BackupsStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))
    vi.mocked(api.listBackups).mockClear()

    await store.backupNow()

    expect(api.createBackup).toHaveBeenCalled()
    expect(api.listBackups).toHaveBeenCalled()
    expect(store.backingUp).toBe(false)
  })

  it('backupNow sets a friendly error on 422 (no destination configured)', async () => {
    vi.mocked(api.createBackup).mockRejectedValue(new Error('422 no destination'))
    const store = new BackupsStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))

    await store.backupNow()
    expect(store.error).toMatch(/destination/)
  })

  it('deleteBackup removes then reloads', async () => {
    const store = new BackupsStore()
    await vi.waitFor(() => expect(store.loading).toBe(false))

    await store.deleteBackup('arrgh-backup-2026-01-01T00-00-00.db')
    expect(api.deleteBackup).toHaveBeenCalledWith('arrgh-backup-2026-01-01T00-00-00.db')
  })

  it('restore sets a friendly error for backup_too_new', async () => {
    vi.mocked(api.restoreBackup).mockRejectedValue(new Error('422 backup_too_new'))
    const store = new BackupsStore()
    await store.restore('arrgh-backup-2026-01-01T00-00-00.db')
    expect(store.restoreError).toMatch(/newer version/)
    expect(store.restoring).toBe(false)
  })

  it('restore sets a friendly error for invalid_backup', async () => {
    vi.mocked(api.restoreBackup).mockRejectedValue(new Error('422 invalid_backup'))
    const store = new BackupsStore()
    await store.restore('junk.db')
    expect(store.restoreError).toMatch(/not a valid backup/)
  })

  it('restoreUpload calls the upload client fn', async () => {
    const store = new BackupsStore()
    const file = new File([new Uint8Array([1, 2, 3])], 'mine.db')
    await store.restoreUpload(file)
    expect(api.restoreBackupUpload).toHaveBeenCalledWith(file)
  })
})
