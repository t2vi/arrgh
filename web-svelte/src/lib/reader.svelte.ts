import { api } from './api'
import type { AppSettings, Chapter, ReadProgress, Title } from './types'

export class ReaderStore {
  #chapterId = $state<string | undefined>(undefined)

  page = $state(0)
  lastPage = $state<number | null>(null)
  #modeOverride = $state<'paged' | 'scroll' | null>(null)
  novelProgress = $state(0)

  chapter = $state<Chapter | undefined>(undefined)
  #readProgress = $state<ReadProgress | null>(null)
  #settings = $state<AppSettings | undefined>(undefined)
  manga = $state<Title | undefined>(undefined)
  chapters = $state<Chapter[]>([])
  novelContent = $state<string | null>(null)
  novelError = $state(false)

  get chapterId() {
    return this.#chapterId
  }

  get effectiveMode(): 'paged' | 'scroll' {
    return this.#modeOverride ?? ((this.manga?.reader_mode as 'paged' | 'scroll' | null) ?? this.#settings?.reader_mode ?? 'paged')
  }

  get total() {
    const known = this.chapter?.page_count ?? 0
    return known > 0 ? known : this.lastPage
  }
  get totalLabel() {
    return this.total != null ? String(this.total) : '?'
  }
  get atEnd() {
    return this.total != null && this.page >= this.total - 1
  }

  get prevChapter(): Chapter | null {
    const i = this.chapters.findIndex((c) => c.id === this.#chapterId)
    return i > 0 ? this.chapters[i - 1] : null
  }
  get nextChapter(): Chapter | null {
    const i = this.chapters.findIndex((c) => c.id === this.#chapterId)
    return i >= 0 && i < this.chapters.length - 1 ? this.chapters[i + 1] : null
  }

  get isNovel() {
    return this.chapter?.chapter_format === 'text'
  }
  get progress() {
    if (this.isNovel) return this.novelProgress
    return this.total != null && this.total > 0 ? (this.page + 1) / this.total : 0
  }

  constructor() {
    $effect(() => {
      const id = this.#chapterId
      if (!id) return
      api.getChapter(id).then((c) => (this.chapter = c)).catch(() => {})
      api.getProgress(id).then((p) => (this.#readProgress = p)).catch(() => {})
      api.getSettings().then((s) => (this.#settings = s)).catch(() => {})
    })

    $effect(() => {
      const id = this.#chapterId
      if (!id || !this.chapter || this.chapter.chapter_format !== 'text') return
      this.novelContent = null
      this.novelError = false
      api
        .getChapterText(id)
        .then((r) => (this.novelContent = r.content))
        .catch(() => (this.novelError = true))
    })

    $effect(() => {
      const titleId = this.chapter?.title_id
      if (!titleId) return
      api.getTitle(titleId).then((m) => (this.manga = m)).catch(() => {})
      api
        .listChapters(titleId)
        .then((list) => (this.chapters = [...list].sort((a, b) => a.number - b.number)))
        .catch(() => {})
    })

    $effect(() => {
      const rp = this.#readProgress
      if (rp?.current_page != null && !rp.completed) this.page = rp.current_page
    })

    $effect(() => {
      if (this.effectiveMode !== 'paged') return
      const onKey = (e: KeyboardEvent) => {
        if (e.key === 'ArrowRight' || e.key === 'ArrowDown') this.goTo(this.page + 1)
        if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') this.goTo(this.page - 1)
      }
      window.addEventListener('keydown', onKey)
      return () => window.removeEventListener('keydown', onKey)
    })
  }

  load(chapterId: string) {
    this.#chapterId = chapterId
  }

  setLastPage(p: number) {
    this.lastPage = p
  }

  goTo(p: number) {
    const clamped = this.total != null ? Math.max(0, Math.min(p, this.total - 1)) : Math.max(0, p)
    this.page = clamped
    const completed = this.total != null && clamped >= this.total - 1
    if (this.#chapterId) api.updateProgress(this.#chapterId, clamped, completed).catch(() => {})
  }

  toggleMode() {
    this.#modeOverride = (this.#modeOverride ?? this.effectiveMode) === 'paged' ? 'scroll' : 'paged'
  }

  markNovelRead() {
    if (this.#chapterId) api.updateProgress(this.#chapterId, 0, true).catch(() => {})
  }

  setNovelProgress(pct: number) {
    this.novelProgress = pct
  }
}
