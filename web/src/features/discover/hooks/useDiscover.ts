import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { api, type SearchResult } from '@/api'
import { ROUTES } from '@/lib/routes'

export interface DiscoverHandle {
  query: string
  setQuery: (v: string) => void
  data: SearchResult[] | undefined
  isFetching: boolean
  searchError: string | null
  addError: string | null
  addingId: string | null
  added: Map<string, string>
  submit: () => void
  handleAdd: (result: SearchResult) => void
  navigate: ReturnType<typeof useNavigate>
}

export function useDiscover(): DiscoverHandle {
  const [query, setQuery] = useState('')
  const [submitted, setSubmitted] = useState('')
  const navigate = useNavigate()

  const [added, setAdded] = useState<Map<string, string>>(new Map())
  const [addError, setAddError] = useState<string | null>(null)
  const [addingId, setAddingId] = useState<string | null>(null)

  const [data, setData] = useState<SearchResult[] | undefined>()
  const [isFetching, setIsFetching] = useState(false)
  const [searchError, setSearchError] = useState<string | null>(null)

  useEffect(() => {
    if (!submitted) return
    setIsFetching(true)
    setSearchError(null)
    api.searchManga(submitted)
      .then((r) => { setData(r); setSearchError(null) })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : ''
        if (msg.includes('502')) {
          setSearchError('MangaUpdates search failed. Check your connection.')
        } else {
          setSearchError('Search failed. Is the server running?')
        }
      })
      .finally(() => setIsFetching(false))
  }, [submitted])

  async function handleAdd(result: SearchResult) {
    setAddError(null)
    setAddingId(result.mangaupdates_id)
    try {
      const manga = await api.addManga(result)
      setAdded((prev) => {
        const next = new Map(prev)
        next.set(result.mangaupdates_id, manga.id)
        return next
      })
      navigate(ROUTES.library)
    } catch (err) {
      setAddError(err instanceof Error ? err.message : 'Failed to add manga')
    } finally {
      setAddingId(null)
    }
  }

  function submit() {
    const q = query.trim()
    if (q) setSubmitted(q)
  }

  return {
    query, setQuery,
    data, isFetching, searchError,
    addError, addingId, added,
    submit, handleAdd, navigate,
  }
}
