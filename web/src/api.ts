import type { AppSettings, Chapter, Manga, PaginatedManga, ReadProgress } from './types'

export interface SourceAlternative {
  source: string
  source_name: string
  id: string
  cover_url: string | null
  status: string
}

export interface SearchResult {
  id: string
  source: string
  source_name: string
  title: string
  description: string | null
  cover_url: string | null
  status: string
  author: string | null
  year: number | null
  tags: string | null
  content_type: string
  in_library: boolean
  library_id?: string
  alternatives: SourceAlternative[]
}

export interface MangaDetailResult {
  description: string | null
  cover_url: string | null
  chapter_count: number
  tags: string | null
}

export interface QueueItem {
  id: string
  chapter_id: string
  manga_title: string
  chapter_num: number
  status: 'pending' | 'downloading' | 'done' | 'error' | 'cancelled'
  error: string | null
  pages_downloaded: number
  pages_total: number
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
  role: string
  allow_explicit: boolean
}

export interface MeResponse {
  id: string
  username: string
  role: string
  allow_explicit: boolean
}

export interface UserListItem {
  id: string
  username: string
  role: string
  allow_explicit: boolean
  created_at: string
}

export interface SourceRow {
  id: string
  name: string
  base_url: string
  has_api_key: boolean
  content_types: string[]
  enabled: boolean
  is_community: boolean
}

export interface PluginIndexEntry {
  id: string
  name: string
  description: string | null
  version: string
  download_url: string | null
  bundled: boolean | null
  default_explicit: boolean
  content_types: string[]
}

// ——— Token storage ———

export function getToken(): string | null {
  return localStorage.getItem('arrgh_token')
}

export function setToken(token: string, username: string, role: string, allow_explicit: boolean): void {
  localStorage.setItem('arrgh_token', token)
  localStorage.setItem('arrgh_username', username)
  localStorage.setItem('arrgh_role', role)
  localStorage.setItem('arrgh_allow_explicit', String(allow_explicit))
}

export function clearToken(): void {
  localStorage.removeItem('arrgh_token')
  localStorage.removeItem('arrgh_username')
  localStorage.removeItem('arrgh_role')
  localStorage.removeItem('arrgh_allow_explicit')
}

export function getUsername(): string | null {
  return localStorage.getItem('arrgh_username')
}

export function getRole(): string {
  return localStorage.getItem('arrgh_role') ?? 'member'
}

export function getAllowExplicit(): boolean {
  return localStorage.getItem('arrgh_allow_explicit') === 'true'
}

export function isAdmin(): boolean {
  return getRole() === 'admin'
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
  if (res.status === 204 || res.status === 202 || res.status === 201) return undefined as T
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
  changePassword: (password: string) => patch<void>('/api/auth/me', { password }),

  // Users (admin only)
  listUsers: () => get<UserListItem[]>('/api/users'),
  createUser: (username: string, password: string) =>
    post<void>('/api/users', { username, password }),
  patchUser: (id: string, body: { role?: string; allow_explicit?: boolean; password?: string }) =>
    patch<void>(`/api/users/${id}`, body),
  deleteUser: (id: string) => del(`/api/users/${id}`),

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
  getChapterText: (id: string) => get<{ content: string }>(`/api/chapters/${id}/text`),
  downloadChapter: (id: string) => post<void>(`/api/chapters/${id}/download`),

  getProgress: (chapterId: string) => get<ReadProgress>(`/api/progress/${chapterId}`),
  getMangaProgress: (mangaId: string) => get<ReadProgress[]>(`/api/progress/manga/${mangaId}`),
  updateProgress: (chapterId: string, currentPage: number, completed: boolean) =>
    put<ReadProgress>(`/api/progress/${chapterId}`, { current_page: currentPage, completed }),

  searchManga: (q: string, contentType?: string) =>
    get<SearchResult[]>('/api/discover', { q, ...(contentType ? { content_type: contentType } : {}) }),

  getTrending: () => get<SearchResult[]>('/api/discover/trending'),

  getDiscoverDetail: (source: string, sourceId: string, title?: string) =>
    get<MangaDetailResult>('/api/discover/detail', { source, source_id: sourceId, ...(title ? { title } : {}) }),

  addManga: (result: SearchResult & { source_id?: string }) =>
    post<Manga>('/api/discover/add', {
      source: result.source,
      source_id: result.source_id ?? result.id,
      title: result.title,
      description: result.description,
      cover_url: result.cover_url,
      status: result.status,
      author: result.author,
      year: result.year,
      tags: result.tags,
      content_type: result.content_type,
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

  setMangaExplicit: (id: string, value: boolean) =>
    patch<void>(`/api/manga/${id}`, { is_explicit: value }),

  setMangaCoverUrl: (id: string, url: string) =>
    patch<void>(`/api/manga/${id}`, { cover_url: url }),

  getSettings: () => get<AppSettings>('/api/settings'),
  saveSettings: (s: Partial<AppSettings>) => post<AppSettings>('/api/settings', s),

  pageUrl: (chapterId: string, page: number) => `${base()}/api/media/page/${chapterId}/${page}`,
  coverUrl: (mangaId: string) => `${base()}/api/media/cover/${mangaId}`,
  proxyImageUrl: (url: string) =>
    url.startsWith('/api/') ? url : `/api/media/proxy?url=${encodeURIComponent(url)}`,

  // Sources (admin only)
  listSources: () => get<SourceRow[]>('/api/sources'),
  addSource: (base_url: string, api_key?: string) =>
    post<void>('/api/sources', { base_url, api_key: api_key || undefined }),
  patchSource: (id: string, enabled: boolean) =>
    patch<void>(`/api/sources/${id}`, { enabled }),
  deleteSource: (id: string) => del(`/api/sources/${id}`),

  // Plugin index (admin only)
  listPluginIndex: () => get<PluginIndexEntry[]>('/api/plugins/index'),
  installPlugin: (plugin_id: string) => post<void>('/api/plugins/install', { plugin_id }),
  deletePlugin: (plugin_id: string) => del(`/api/plugins/${encodeURIComponent(plugin_id)}`),
}
