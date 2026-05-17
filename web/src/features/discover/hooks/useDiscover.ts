import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, type SearchResult } from '@/api'
import { ROUTES } from '@/lib/routes'

export interface DiscoverHandle {
  query: string
  setQuery: (v: string) => void
  contentType: string | undefined
  setContentType: (v: string | undefined) => void
  availableContentTypes: Set<string>
  data: SearchResult[] | undefined
  isFetching: boolean
  searchError: string | null
  addError: string | null
  addingKey: string | null
  added: Map<string, string>
  submit: () => void
  handleAdd: (result: SearchResult, alt?: { source: string; id: string; source_name: string }) => void
  navigate: ReturnType<typeof useNavigate>
}

export function useDiscover(): DiscoverHandle {
  const [query, setQuery] = useState('')
  const [submitted, setSubmitted] = useState('')
  const [contentType, setContentType] = useState<string | undefined>()
  const [availableContentTypes, setAvailableContentTypes] = useState<Set<string>>(new Set())
  const navigate = useNavigate()

  const [added, setAdded] = useState<Map<string, string>>(new Map())
  const [addError, setAddError] = useState<string | null>(null)
  const [addingKey, setAddingKey] = useState<string | null>(null)

  const [data, setData] = useState<SearchResult[] | undefined>()
  const [isFetching, setIsFetching] = useState(false)
  const [searchError, setSearchError] = useState<string | null>(null)

  useEffect(() => {
    api.listSources()
      .then((sources) => {
        const types = new Set<string>()
        sources.filter((s) => s.enabled).forEach((s) => s.content_types.forEach((t) => types.add(t)))
        setAvailableContentTypes(types)
        setContentType((ct) => (ct && !types.has(ct) ? undefined : ct))
      })
      .catch(() => {})
  }, [])

  useEffect(() => {
    if (!submitted) return
    setIsFetching(true)
    setSearchError(null)
    api.searchManga(submitted, contentType)
      .then((r) => { setData(r); setSearchError(null) })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : ''
        if (msg.includes('502')) {
          setSearchError('All sources failed to respond. Check that your plugins are running.')
        } else {
          setSearchError('Search failed. Is the server running?')
        }
      })
      .finally(() => setIsFetching(false))
  }, [submitted, contentType])

  async function handleAdd(result: SearchResult, alt?: { source: string; id: string; source_name: string }) {
    const source = alt?.source ?? result.source
    const sourceId = alt?.id ?? result.id
    const key = `${source}:${sourceId}`
    setAddError(null)
    setAddingKey(key)
    try {
      const manga = await api.addManga({ ...result, source, source_id: sourceId })
      setAdded((prev) => {
        const next = new Map(prev)
        next.set(`${result.source}:${result.id}`, manga.id)
        if (alt) next.set(key, manga.id)
        return next
      })
      navigate(ROUTES.manga(manga.id))
    } catch (err) {
      setAddError(err instanceof Error ? err.message : 'Failed to add manga')
    } finally {
      setAddingKey(null)
    }
  }

  function submit() {
    const q = query.trim()
    if (q) setSubmitted(q)
  }

  return {
    query, setQuery,
    contentType, setContentType,
    availableContentTypes,
    data, isFetching, searchError,
    addError, addingKey, added,
    submit, handleAdd, navigate,
  }
}
