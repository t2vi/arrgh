import { api, type SearchResult } from './api'
import { router } from './router.svelte'
import { ROUTES } from './routes'

export class DiscoverStore {
  query = $state('')
  #submitted = $state('')

  data = $state<SearchResult[] | undefined>(undefined)
  isFetching = $state(false)
  searchError = $state<string | null>(null)
  contentTypeFilter = $state<string | undefined>(undefined)

  /** True for ~700ms after results arrive — show SearchProgress in completed state. */
  showProgress = $state(false)
  /** Sources that contributed results; defined only when showProgress is true. */
  completedSources = $state<Set<string> | undefined>(undefined)
  #prevFetching = false

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
      this.isFetching = true
      this.searchError = null
      this.completedSources = undefined
      this.contentTypeFilter = undefined
      api
        .searchManga(q)
        .then((r) => {
          this.data = r
          this.searchError = null
        })
        .catch((err: unknown) => {
          const msg = err instanceof Error ? err.message : ''
          this.searchError = msg.includes('502')
            ? 'Discovery failed. Check your connection or server status.'
            : 'Search failed. Is the server running?'
        })
        .finally(() => (this.isFetching = false))
    })

    // When fetch completes with results, briefly show the green-pill "done" state before
    // revealing the results list — gives the user a clear signal which sources responded.
    $effect(() => {
      const fetching = this.isFetching
      const data = this.data
      if (this.#prevFetching && !fetching && data && data.length > 0) {
        const sources = new Set(data.map((r) => r.source))
        this.completedSources = sources
        this.showProgress = true
        this.#prevFetching = false
        const t = setTimeout(() => (this.showProgress = false), 700)
        return () => clearTimeout(t)
      }
      this.#prevFetching = fetching
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
