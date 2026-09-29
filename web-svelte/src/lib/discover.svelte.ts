import { untrack } from 'svelte'
import { api, type SearchResult, type SourceInfo, type SourceStatus } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'

const SEARCHING = 'searching'
const DISCOVERY_FAILED = 'Discovery failed. Check your connection or server status.'
const SEARCH_FAILED = 'Search failed. Is the server running?'

export type SourceProgress = { status: typeof SEARCHING | SourceStatus; count?: number }

export class DiscoverStore {
  query = $state('')
  #submitted = $state('')

  data = $state<SearchResult[] | undefined>(undefined)
  isFetching = $state(false)
  searchError = $state<string | null>(null)
  contentTypeFilter = $state<string | undefined>(undefined)

  /** Queried sources, from the stream's `sources` event (server-owned list). */
  sources = $state<SourceInfo[]>([])
  sourceState = $state<Map<string, SourceProgress>>(new Map())
  /** True while searching and for ~700ms after `done`, so the settled pills stay visible briefly. */
  showProgress = $state(false)
  #gen = 0

  added = $state<Map<string, string>>(new Map())
  addError = $state<string | null>(null)
  addingId = $state<string | null>(null)

  get availableTypes(): Set<string> {
    if (!this.data) return new Set()
    return new Set(this.data.map((r) => r.content_type))
  }

  get filteredData(): SearchResult[] | undefined {
    if (!this.data) return undefined
    if (!this.contentTypeFilter) return this.data
    return this.data.filter((r) => r.content_type === this.contentTypeFilter)
  }

  constructor() {
    $effect(() => {
      const q = this.#submitted
      if (!q) return
      const gen = ++this.#gen
      const ctrl = new AbortController()
      let collapse: ReturnType<typeof setTimeout> | undefined
      this.isFetching = true
      this.showProgress = true
      this.searchError = null
      this.contentTypeFilter = undefined
      this.data = undefined
      this.sources = []
      this.sourceState = new Map()

      // Buffered here, not committed to `this.data` until the stream's `done` —
      // results (including source-fallback) only render once every leg has
      // actually settled, not mid-search.
      let pending: SearchResult[] | undefined

      const finish = (error: string | null) => {
        this.data = pending
        this.isFetching = false
        this.searchError = error
        collapse = setTimeout(() => (this.showProgress = false), 700)
      }

      // untrack: event handlers read state (sourceState, isFetching) and may run
      // synchronously; they must not become dependencies of this effect.
      untrack(() => api
        .searchMangaStream(
          q,
          (e) => {
            if (gen !== this.#gen) return
            if (e.type === 'sources') {
              this.sources = e.sources
              this.sourceState = new Map(e.sources.map((s) => [s.key, { status: SEARCHING }]))
            } else if (e.type === 'source') {
              this.sourceState = new Map(this.sourceState).set(e.key, { status: e.status, count: e.count })
              pending = e.results
            } else {
              finish(e.ok ? null : DISCOVERY_FAILED)
            }
          },
          ctrl.signal,
        )
        .then(() => {
          // Stream closed without a `done` event (connection dropped).
          if (gen === this.#gen && this.isFetching) finish(pending ? null : SEARCH_FAILED)
        })
        .catch((err: unknown) => {
          if (gen !== this.#gen || ctrl.signal.aborted) return
          const msg = err instanceof Error ? err.message : ''
          finish(msg.includes('502') ? DISCOVERY_FAILED : SEARCH_FAILED)
        }))

      // New search or leaving the page: stop reading the old stream.
      return () => {
        ctrl.abort()
        clearTimeout(collapse)
      }
    })
  }

  setContentTypeFilter(v: string | undefined) {
    this.contentTypeFilter = this.contentTypeFilter === v ? undefined : v
  }

  async handleAdd(result: SearchResult) {
    this.addError = null
    this.addingId = result.mangaupdates_id
    try {
      const manga = await api.addTitle(result)
      const next = new Map(this.added)
      next.set(result.mangaupdates_id, manga.id)
      this.added = next
      router.navigate(ROUTES.library)
    } catch (err) {
      this.addError = err instanceof Error ? err.message : 'Failed to add manga'
    } finally {
      this.addingId = null
    }
  }

  submit() {
    const q = this.query.trim()
    if (q) this.#submitted = q
  }
}
