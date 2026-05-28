export interface Title {
  id: string
  title: string
  description: string | null
  cover_url: string | null
  status: string
  is_local: boolean
  local_path: string | null
  author: string | null
  year: number | null
  tags: string | null
  sync_status: string
  content_type: string
  auto_download: boolean | null
  reader_mode: string | null
  download_dir: string | null
  is_explicit: boolean
  has_sync_warnings: boolean
  created_at: string
  updated_at: string
  // Present on list endpoint only
  total_chapters?: number
  downloaded_chapters?: number
  chapters_read?: number
}

export interface PaginatedTitle {
  items: Title[]
  total: number
  page: number
  limit: number
}

export interface Chapter {
  id: string
  title_id: string
  title: string | null
  number: number
  volume: number | null
  local_path: string | null
  page_count: number
  downloaded: boolean
  has_sources: boolean
  chapter_format: string
  created_at: string
}

export interface AppSettings {
  download_workers: number
  index_interval_hours: number
  auto_download: boolean
  reader_mode: 'paged' | 'scroll'
  download_dir: string
  trending_per_source: number
  check_for_updates: boolean
}

export interface SyncLogEntry {
  id: string
  message: string
  created_at: string
}

export interface ReadProgress {
  id: string
  chapter_id: string
  current_page: number
  completed: boolean
  updated_at: string
}
