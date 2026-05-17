import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { AlignJustify, BookOpen } from 'lucide-react'
import { api } from '@/api'

export interface ReaderHandle {
  chapterId: string | undefined
  navigate: ReturnType<typeof useNavigate>
  page: number
  lastPage: number | null
  setLastPage: (p: number) => void
  effectiveMode: 'paged' | 'scroll'
  chapter: Awaited<ReturnType<typeof api.getChapter>> | undefined
  total: number | null
  totalLabel: string
  atEnd: boolean
  isNovel: boolean
  novelContent: string | null
  novelError: boolean
  goTo: (p: number) => void
  markNovelRead: () => void
  toggleMode: () => void
  ModeIcon: typeof AlignJustify
}

export function useReader(): ReaderHandle {
  const { chapterId } = useParams<{ chapterId: string }>()
  const navigate = useNavigate()
  const [page, setPage] = useState(0)
  const [lastPage, setLastPage] = useState<number | null>(null)
  const [modeOverride, setModeOverride] = useState<'paged' | 'scroll' | null>(null)

  const [chapter, setChapter] = useState<Awaited<ReturnType<typeof api.getChapter>> | undefined>()
  const [progress, setProgress] = useState<Awaited<ReturnType<typeof api.getProgress>> | null>(null)
  const [settings, setSettings] = useState<Awaited<ReturnType<typeof api.getSettings>> | undefined>()
  const [manga, setManga] = useState<Awaited<ReturnType<typeof api.getManga>> | undefined>()
  const [novelContent, setNovelContent] = useState<string | null>(null)
  const [novelError, setNovelError] = useState(false)

  useEffect(() => {
    if (!chapterId) return
    api.getChapter(chapterId).then(setChapter).catch(() => {})
    api.getProgress(chapterId).then(setProgress).catch(() => {})
    api.getSettings().then(setSettings).catch(() => {})
  }, [chapterId])

  useEffect(() => {
    if (!chapterId || !chapter || chapter.chapter_format !== 'text') return
    setNovelContent(null)
    setNovelError(false)
    api.getChapterText(chapterId)
      .then((r) => setNovelContent(r.content))
      .catch(() => setNovelError(true))
  }, [chapterId, chapter?.chapter_format])

  useEffect(() => {
    if (!chapter?.manga_id) return
    api.getManga(chapter.manga_id).then(setManga).catch(() => {})
  }, [chapter?.manga_id])

  const effectiveMode: 'paged' | 'scroll' =
    modeOverride ?? (manga?.reader_mode as 'paged' | 'scroll' | null) ?? settings?.reader_mode ?? 'paged'

  useEffect(() => {
    if (progress?.current_page != null && !progress.completed) {
      setPage(progress.current_page)
    }
  }, [progress])

  const knownTotal = chapter?.page_count ?? 0
  const total = knownTotal > 0 ? knownTotal : (lastPage != null ? lastPage : null)
  const totalLabel = total != null ? String(total) : '?'
  const atEnd = total != null && page >= total - 1

  function goTo(p: number) {
    const clamped = total != null
      ? Math.max(0, Math.min(p, total - 1))
      : Math.max(0, p)
    setPage(clamped)
    const completed = total != null && clamped >= total - 1
    api.updateProgress(chapterId!, clamped, completed).catch(() => {})
  }

  useEffect(() => {
    if (effectiveMode !== 'paged') return
    function onKey(e: KeyboardEvent) {
      if (e.key === 'ArrowRight' || e.key === 'ArrowDown') goTo(page + 1)
      if (e.key === 'ArrowLeft'  || e.key === 'ArrowUp')   goTo(page - 1)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  function toggleMode() {
    setModeOverride((m) => (m ?? effectiveMode) === 'paged' ? 'scroll' : 'paged')
  }

  const isNovel = chapter?.chapter_format === 'text'
  const ModeIcon = effectiveMode === 'scroll' ? BookOpen : AlignJustify

  function markNovelRead() {
    api.updateProgress(chapterId!, 0, true).catch(() => {})
  }

  return {
    chapterId, navigate,
    page, lastPage, setLastPage,
    effectiveMode, chapter,
    total, totalLabel, atEnd,
    isNovel, novelContent, novelError,
    goTo, markNovelRead, toggleMode, ModeIcon,
  }
}
