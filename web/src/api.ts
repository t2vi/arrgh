import type { AppSettings, Chapter, Manga, PaginatedManga, ReadProgress } from './types'

export interface SearchResult {
  id: string
  source: string
  title: string
  description: string | null
  cover_url: string | null
  status: string
  author: string | null
  year: number | null
  tags: string | null
  in_library: boolean
  library_id?: string
}

export interface MangaDetailResult {
  description: string | null
  cover_url: string | null
  chapter_count: number
}

export interface QueueItem {
  id: string
  chapter_id: string
  manga_title: string
  chapter_num: number
  status: 'pending' | 'downloading' | 'done' | 'error' | 'cancelled'
  error: string | null
  created_at: string
  updated_at: string
}

export interface ContinueItem {
  manga_id: string
  manga_title: string
  cover_url: string | null
  chapter_id: string
  chapter_number: number
  chapters_read: number
  total_chapters: number
}

export interface NewReleaseItem {
  chapter_id: string
  chapter_number: number
  chapter_title: string | null
  chapter_created_at: string
  downloaded: boolean
  manga_id: string
  manga_title: string
  cover_url: string | null
}

export interface AuthStatus {
  needs_setup: boolean
}

export interface AuthResponse {
  token: string
  username: string
  user_id: string
}

export interface MeResponse {
  id: string
  username: string
}

// ——— Token storage ———

export function getToken(): string | null {
  return localStorage.getItem('arrgh_token')
}

export function setToken(token: string, username: string): void {
  localStorage.setItem('arrgh_token', token)
  localStorage.setItem('arrgh_username', username)
}

export function clearToken(): void {
  localStorage.removeItem('arrgh_token')
  localStorage.removeItem('arrgh_username')
}

export function getUsername(): string | null {
  return localStorage.getItem('arrgh_username')
}

// ——— HTTP helpers ———

function base(): string {
  return localStorage.getItem('serverUrl') ?? ''
}

function authHeaders(extra?: Record<string, string>): Record<string, string> {
  const token = getToken()
  return { ...(token ? { Authorization: `Bearer ${token}` } : {}), ...extra }
}

function handle401(status: number) {
  if (status === 401) {
    clearToken()
    window.dispatchEvent(new Event('arrgh:unauthorized'))
  }
}

async function get<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(base() + path, window.location.href)
  if (params) {
    Object.entries(params).forEach(([k, v]) => url.searchParams.set(k, v))
  }
  const res = await fetch(url, { headers: authHeaders() })
  handle401(res.status)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  return res.json() as Promise<T>
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(base() + path, {
    method: 'POST',
    headers: authHeaders(body != null ? { 'Content-Type': 'application/json' } : {}),
    body: body != null ? JSON.stringify(body) : undefined,
  })
  handle401(res.status)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  if (res.status === 204 || res.status === 202) return undefined as T
  return res.json() as Promise<T>
}

async function put<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(base() + path, {
    method: 'PUT',
    headers: authHeaders({ 'Content-Type': 'application/json' }),
    body: JSON.stringify(body),
  })
  handle401(res.status)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  return res.json() as Promise<T>
}

async function del(path: string, params?: Record<string, string>): Promise<void> {
  const url = new URL(base() + path, window.location.href)
  if (params) Object.entries(params).forEach(([k, v]) => url.searchParams.set(k, v))
  await fetch(url, { method: 'DELETE', headers: authHeaders() })
}

async function patch<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(base() + path, {
    method: 'PATCH',
    headers: authHeaders({ 'Content-Type': 'application/json' }),
    body: JSON.stringify(body),
  })
  handle401(res.status)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
  if (res.status === 204) return undefined as T
  return res.json() as Promise<T>
}

// ——— API ———

export const api = {
  // Auth
  authStatus: () => get<AuthStatus>('/api/auth/status'),
  register: (username: string, password: string) =>
    post<AuthResponse>('/api/auth/register', { username, password }),
  login: (username: string, password: string) =>
    post<AuthResponse>('/api/auth/login', { username, password }),
  me: () => get<MeResponse>('/api/auth/me'),

  getNewReleases: () => get<NewReleaseItem[]>('/api/manga/new-releases'),
  getContinueReading: () => get<ContinueItem[]>('/api/progress/continue'),

  // Manga
  listManga: (page = 1, search?: string) =>
    get<PaginatedManga>('/api/manga', {
      page: String(page),
      ...(search ? { search } : {}),
    }),

  getManga: (id: string) => get<Manga>(`/api/manga/${id}`),
  syncManga: (id: string) => post<void>(`/api/manga/${id}/sync`),

  listChapters: (mangaId: string) => get<Chapter[]>(`/api/chapters/manga/${mangaId}`),
  getChapter: (id: string) => get<Chapter>(`/api/chapters/${id}`),
  downloadChapter: (id: string) => post<void>(`/api/chapters/${id}/download`),

  getProgress: (chapterId: string) => get<ReadProgress>(`/api/progress/${chapterId}`),
  getMangaProgress: (mangaId: string) => get<ReadProgress[]>(`/api/progress/manga/${mangaId}`),
  updateProgress: (chapterId: string, currentPage: number, completed: boolean) =>
    put<ReadProgress>(`/api/progress/${chapterId}`, { current_page: currentPage, completed }),

  searchManga: (q: string) =>
    get<SearchResult[]>('/api/discover', { q }),

  getTrending: () => get<SearchResult[]>('/api/discover/trending'),

  getDiscoverDetail: (source: string, sourceId: string) =>
    get<MangaDetailResult>('/api/discover/detail', { source, source_id: sourceId }),

  addManga: (result: SearchResult) =>
    post<Manga>('/api/discover/add', {
      source: result.source,
      source_id: result.id,
      title: result.title,
      description: result.description,
      cover_url: result.cover_url,
      status: result.status,
      author: result.author,
      year: result.year,
      tags: result.tags,
    }),

  getQueue: () => get<QueueItem[]>('/api/queue'),
  getMangaQueue: (mangaId: string) => get<QueueItem[]>(`/api/queue/manga/${mangaId}`),

  removeManga: (id: string, deleteFiles = false) =>
    del(`/api/manga/${id}`, deleteFiles ? { delete_files: 'true' } : undefined),

  removeFromQueue: (id: string) => del(`/api/queue/${id}`),
  clearCompletedQueue: () => del('/api/queue/completed'),

  setMangaReaderMode: (id: string, value: string | null) =>
    patch<void>(`/api/manga/${id}`, { reader_mode: value }),

  setMangaAutoDownload: (id: string, value: boolean | null) =>
    patch<void>(`/api/manga/${id}`, { auto_download: value }),

  setMangaDownloadDir: (id: string, value: string | null) =>
    patch<void>(`/api/manga/${id}`, { download_dir: value }),

  getSettings: () => get<AppSettings>('/api/settings'),
  saveSettings: (s: Partial<AppSettings>) => post<AppSettings>('/api/settings', s),

  pageUrl: (chapterId: string, page: number) => `${base()}/api/media/page/${chapterId}/${page}`,
  coverUrl: (mangaId: string) => `${base()}/api/media/cover/${mangaId}`,
  proxyImageUrl: (url: string) => `/api/media/proxy?url=${encodeURIComponent(url)}`,
}
