import { api, type BackupRow } from './api'

export class BackupsStore {
  backups = $state<BackupRow[]>([])
  loading = $state(true)
  backingUp = $state(false)
  restoring = $state(false)
  error = $state('')
  restoreError = $state('')

  constructor() {
    this.load()
  }

  async load() {
    this.loading = true
    try {
      this.backups = await api.listBackups()
    } catch {
      this.error = 'Failed to load backups.'
    } finally {
      this.loading = false
    }
  }

  async backupNow() {
    this.backingUp = true
    this.error = ''
    try {
      await api.createBackup()
      await this.load()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      this.error = msg.includes('422')
        ? 'Set a backup destination in Settings first.'
        : 'Backup failed.'
    } finally {
      this.backingUp = false
    }
  }

  async deleteBackup(filename: string) {
    try {
      await api.deleteBackup(filename)
      await this.load()
    } catch {
      this.error = 'Failed to delete backup.'
    }
  }

  async restore(filename: string) {
    this.restoring = true
    this.restoreError = ''
    try {
      await api.restoreBackup(filename)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      this.restoreError = msg.includes('backup_too_new')
        ? 'This backup is from a newer version of *ARRgh — upgrade before restoring it.'
        : msg.includes('invalid_backup')
          ? 'This is not a valid backup file.'
          : 'Restore failed.'
    } finally {
      this.restoring = false
    }
  }

  async restoreUpload(file: File) {
    this.restoring = true
    this.restoreError = ''
    try {
      await api.restoreBackupUpload(file)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : ''
      this.restoreError = msg.includes('backup_too_new')
        ? 'This backup is from a newer version of *ARRgh — upgrade before restoring it.'
        : msg.includes('invalid_backup')
          ? 'This is not a valid backup file.'
          : 'Restore failed.'
    } finally {
      this.restoring = false
    }
  }
}
