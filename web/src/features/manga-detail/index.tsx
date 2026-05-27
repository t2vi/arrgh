import React from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import {
  Download, RefreshCw, Loader2, X, ArrowUp, ArrowDown, Trash2, ChevronDown, AlertTriangle,
} from 'lucide-react'
import { api, isAdmin } from '@/api'
import { Button } from '@/components/ui/button'
import { ContentTypePill } from '@/components/ContentTypePill'
import { ROUTES } from '@/lib/routes'
import { cn } from '@/lib/utils'
import { useMangaDetail, CHAPTERS_PREVIEW, type FilterMode } from './hooks/useMangaDetail'
import { ChapterRow } from './components/ChapterRow'
import {
  SectionHeading, CoverImg, ChapterListSkeleton,
  ReaderModeCard, AutoDownloadCard, ExplicitCard, DownloadDirCard, CoverUrlCard, ContentTypeCard,
} from './components/SidebarCards'

export default function MangaDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const h = useMangaDetail(id)
  const { manga, loadingManga, isSyncing, isRemoteSource } = h

  return (
    <div className="flex flex-col h-full overflow-auto">

      {/* ── Hero ── */}
      {loadingManga ? (
        <div className="h-64 bg-muted animate-pulse shrink-0" />
      ) : manga && (
        <div className="relative shrink-0" style={{ minHeight: 272 }}>
          <div className="absolute inset-0 overflow-hidden">
            <div
              className="absolute inset-0 scale-110"
              style={{
                backgroundImage: `url(${h.coverSrc})`,
                backgroundSize: 'cover',
                backgroundPosition: 'center top',
                filter: 'blur(28px)',
                opacity: 0.35,
              }}
            />
            <div className="absolute inset-0 bg-gradient-to-b from-background/50 via-background/75 to-background" />
          </div>

          <div className="relative z-10 flex items-end gap-6 px-6 pt-10 pb-6 max-w-5xl mx-auto">
            <CoverImg coverUrl={manga.cover_url} mangaId={manga.id} />

            <div className="flex-1 min-w-0 pb-1 space-y-2.5">
              <div className="flex gap-1.5 flex-wrap">
                <ContentTypePill type={manga.content_type} />
                {h.tags.slice(0, 5).map((tag) => (
                  <span
                    key={tag}
                    className="text-[11px] px-2 py-0.5 rounded-full bg-white/10 border border-white/10 text-foreground/80 font-medium"
                  >
                    {tag}
                  </span>
                ))}
              </div>

              <h1 className="text-3xl font-bold tracking-tight leading-none">{manga.title}</h1>

              <div className="flex items-center gap-2 text-sm text-muted-foreground flex-wrap">
                {manga.author && (
                  <span className="font-medium text-foreground/90">{manga.author}</span>
                )}
                {manga.author && <span className="opacity-40">•</span>}
                <span className="capitalize">{manga.status}</span>
                {isSyncing && (
                  <>
                    <span className="opacity-40">•</span>
                    <span className="flex items-center gap-1 text-primary text-xs">
                      <Loader2 className="w-3 h-3 animate-spin" /> Syncing…
                    </span>
                  </>
                )}
              </div>

              <div className="flex items-center gap-2 pt-0.5">
                {h.resumeChapter && (
                  <Button
                    onClick={() => h.openOrQueue(h.resumeChapter!)}
                    className="gap-2"
                    disabled={h.pendingReadId === h.resumeChapter.id && !h.resumeChapter.downloaded}
                  >
                    {h.pendingReadId === h.resumeChapter.id && !h.resumeChapter.downloaded
                      ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      : <Download className="w-3.5 h-3.5 fill-current" />}
                    {h.readCount > 0 ? `Resume Ch. ${h.resumeChapter.number}` : `Start Ch. ${h.resumeChapter.number}`}
                  </Button>
                )}
                {isRemoteSource && h.streamable > 0 && (
                  <Button
                    variant="secondary"
                    onClick={() => h.downloadAll.mutate()}
                    disabled={h.downloadAll.isPending || h.streamable <= h.activeCount + h.pendingCount}
                    className="gap-2"
                  >
                    <Download className="w-3.5 h-3.5" />
                    Download All
                  </Button>
                )}
                {isRemoteSource && (
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => h.sync.mutate()}
                    disabled={h.sync.isPending || isSyncing}
                    title="Sync chapters"
                  >
                    <RefreshCw className={cn('w-4 h-4', (h.sync.isPending || isSyncing) && 'animate-spin')} />
                  </Button>
                )}
                {manga.has_sync_warnings && (
                  <Button
                    size="icon"
                    onClick={() => h.refreshMetadata.mutate()}
                    disabled={h.refreshMetadata.isPending}
                    title="Source match failed — click to refresh metadata and retry"
                    className="bg-amber-500 hover:bg-amber-600 text-white border-0"
                  >
                    {h.refreshMetadata.isPending
                      ? <Loader2 className="w-4 h-4 animate-spin" />
                      : <AlertTriangle className="w-4 h-4" />}
                  </Button>
                )}
                <div className="relative" ref={h.removeMenuRef as React.RefObject<HTMLDivElement>}>
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => h.setShowRemoveMenu((v) => !v)}
                    title="Remove"
                  >
                    <Trash2 className="w-4 h-4 text-destructive" />
                  </Button>
                  {h.showRemoveMenu && (
                    <div className="absolute right-0 top-full mt-1 w-56 rounded-lg border border-border bg-card shadow-lg z-50 overflow-hidden">
                      <button
                        className="w-full text-left px-4 py-2.5 text-sm hover:bg-accent/60 transition-colors"
                        onClick={async () => { await api.removeTitle(id!, false); navigate(ROUTES.library) }}
                      >
                        Remove from library
                      </button>
                      {isAdmin() && (
                        <button
                          className="w-full text-left px-4 py-2.5 text-sm text-destructive hover:bg-destructive/10 transition-colors"
                          onClick={async () => {
                            if (!window.confirm('Delete all downloaded files for this manga? This cannot be undone.')) return
                            await api.removeTitle(id!, true)
                            navigate(ROUTES.library)
                          }}
                        >
                          Remove + delete files
                        </button>
                      )}
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* ── Body ── */}
      <div className="max-w-5xl w-full mx-auto px-6 py-6">
        <div className="grid grid-cols-[1fr_272px] gap-6 items-start">

          {/* Left: Synopsis + Chapters */}
          <div className="space-y-6 min-w-0">
            {manga?.description && (
              <section>
                <SectionHeading>Synopsis</SectionHeading>
                <div className="rounded-lg bg-card border border-border p-4 mt-3">
                  <p className="text-sm text-muted-foreground leading-relaxed">{manga.description}</p>
                </div>
              </section>
            )}

            <section>
              <div className="flex items-center justify-between gap-3 flex-wrap">
                <SectionHeading>
                  Chapters
                  {h.total > 0 && (
                    <span className="ml-1.5 text-xs font-normal normal-case tracking-normal text-muted-foreground/50">
                      {h.filteredChapters.length !== h.total
                        ? `${h.filteredChapters.length} / ${h.total}`
                        : `(${h.total})`}
                    </span>
                  )}
                </SectionHeading>

                {h.total > 0 && (
                  <div className="flex items-center gap-2">
                    <div className="flex items-center gap-1 bg-muted rounded-lg p-0.5">
                      {(['all', 'downloaded', 'not_downloaded'] as FilterMode[]).map((mode) => (
                        <button
                          key={mode}
                          onClick={() => { h.setFilterMode(mode); h.setShowAll(false) }}
                          className={cn(
                            'text-[11px] font-medium px-2.5 py-1 rounded-md transition-colors capitalize',
                            h.filterMode === mode
                              ? 'bg-card text-foreground shadow-sm'
                              : 'text-muted-foreground hover:text-foreground',
                          )}
                        >
                          {mode === 'not_downloaded' ? 'Not Downloaded' : mode === 'downloaded' ? 'Downloaded' : 'All'}
                        </button>
                      ))}
                    </div>
                    <button
                      onClick={() => h.setSortDir((d) => d === 'desc' ? 'asc' : 'desc')}
                      className="flex items-center gap-1 px-2 py-1.5 rounded-lg bg-muted hover:bg-accent text-muted-foreground hover:text-foreground transition-colors text-[11px] font-medium"
                    >
                      {h.sortDir === 'desc'
                        ? <ArrowDown className="w-3.5 h-3.5" />
                        : <ArrowUp className="w-3.5 h-3.5" />}
                      {h.sortDir === 'desc' ? 'Newest' : 'Oldest'}
                    </button>
                  </div>
                )}
              </div>

              {(h.activeCount > 0 || h.pendingCount > 0) && (
                <div className="flex items-center justify-between mt-3 px-3 py-2 rounded-lg bg-primary/10 border border-primary/20">
                  <div className="flex items-center gap-2 text-xs text-primary">
                    <Loader2 className="w-3.5 h-3.5 animate-spin shrink-0" />
                    <span>
                      {h.activeCount > 0 && `${h.activeCount} downloading`}
                      {h.activeCount > 0 && h.pendingCount > 0 && ' · '}
                      {h.pendingCount > 0 && `${h.pendingCount} queued`}
                    </span>
                  </div>
                  <button
                    onClick={() => h.cancelAll.mutate()}
                    disabled={h.cancelAll.isPending}
                    className="flex items-center gap-1 text-[11px] font-medium text-primary/70 hover:text-destructive transition-colors"
                  >
                    <X className="w-3 h-3" />
                    Cancel all
                  </button>
                </div>
              )}

              <div className="mt-3">
                {h.loadingChapters ? (
                  <ChapterListSkeleton />
                ) : h.total === 0 ? (
                  <p className="text-sm text-muted-foreground py-4 flex items-center gap-2">
                    {isSyncing || h.sync.isPending
                      ? <><RefreshCw className="w-3.5 h-3.5 animate-spin" /> Fetching chapters…</>
                      : <>No chapters.{isRemoteSource && <button onClick={() => h.sync.mutate()} className="underline hover:text-foreground ml-1">Sync.</button>}</>
                    }
                  </p>
                ) : (
                  <>
                    <div className="space-y-1.5">
                      {h.displayed.map((ch) => (
                        <ChapterRow
                          key={ch.id}
                          chapter={ch}
                          progress={h.progressMap.get(ch.id) ?? null}
                          queueItem={h.queueMap.get(ch.id) ?? null}
                          pendingRead={h.pendingReadId === ch.id}
                          onOpen={() => h.openOrQueue(ch)}
                          onCancelDownload={(qid) => h.removeFromQueue.mutate(qid)}
                        />
                      ))}
                    </div>
                    {h.filteredChapters.length > CHAPTERS_PREVIEW && (
                      <button
                        onClick={() => h.setShowAll(!h.showAll)}
                        className="mt-3 w-full text-sm text-muted-foreground hover:text-foreground transition-colors py-2 flex items-center justify-center gap-1.5"
                      >
                        {h.showAll ? 'Show less' : `View all ${h.filteredChapters.length} chapters`}
                        <ChevronDown className={cn('w-4 h-4 transition-transform', h.showAll && 'rotate-180')} />
                      </button>
                    )}
                  </>
                )}
              </div>
            </section>
          </div>

          {/* Right: Sidebar */}
          <div className="space-y-4">
            {h.total > 0 && (
              <div className="rounded-lg bg-card border border-border p-4 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                    Reading Progress
                  </span>
                  <span className="text-sm font-bold">{h.readPct}%</span>
                </div>
                <div className="h-1.5 rounded-full bg-muted overflow-hidden">
                  <div className="h-full bg-primary rounded-full transition-all" style={{ width: `${h.readPct}%` }} />
                </div>
                <p className="text-[11px] text-muted-foreground">
                  Chapter {h.readCount} of {h.total} completed
                </p>
              </div>
            )}

            <div className="rounded-lg bg-card border border-border overflow-hidden">
              <div className="grid grid-cols-2 divide-x divide-y divide-border">
                {[
                  { label: 'Total',      value: h.total > 0 ? `${h.total} ch` : '—' },
                  { label: 'Downloaded', value: h.downloaded > 0 ? `${h.downloaded} ch` : '—', highlight: h.downloaded > 0 },
                  { label: 'Source',     value: manga ? (manga.is_local ? 'local' : 'remote') : '—' },
                  { label: 'Year',       value: manga?.year ? String(manga.year) : '—' },
                ].map(({ label, value, highlight }) => (
                  <div key={label} className="p-3">
                    <p className="text-[10px] text-muted-foreground uppercase tracking-wider">{label}</p>
                    <p className={cn('text-sm font-semibold mt-0.5 capitalize', highlight && 'text-[#34d399]')}>{value}</p>
                  </div>
                ))}
                {(h.activeCount > 0 || h.pendingCount > 0) && <>
                  <div className="p-3">
                    <p className="text-[10px] text-muted-foreground uppercase tracking-wider">Downloading</p>
                    <p className="text-sm font-semibold mt-0.5 text-primary flex items-center gap-1">
                      <Loader2 className="w-3 h-3 animate-spin shrink-0" />{h.activeCount} ch
                    </p>
                  </div>
                  <div className="p-3">
                    <p className="text-[10px] text-muted-foreground uppercase tracking-wider">Queued</p>
                    <p className="text-sm font-semibold mt-0.5 text-muted-foreground flex items-center gap-1">
                      <Download className="w-3 h-3 shrink-0" />{h.pendingCount} ch
                    </p>
                  </div>
                </>}
              </div>
            </div>

            {manga && !manga.is_local && (
              <AutoDownloadCard mangaId={manga.id} value={manga.auto_download} />
            )}
            {manga && <ReaderModeCard mangaId={manga.id} value={manga.reader_mode ?? null} />}
            {manga && <DownloadDirCard mangaId={manga.id} value={manga.download_dir ?? null} />}
            {manga && isAdmin() && (
              <ExplicitCard mangaId={manga.id} value={manga.is_explicit ?? false} />
            )}
            {manga && isAdmin() && (
              <ContentTypeCard mangaId={manga.id} value={manga.content_type} />
            )}
            {manga && isAdmin() && (
              <CoverUrlCard
                mangaId={manga.id}
                value={manga.cover_url?.startsWith('http') ? manga.cover_url : null}
                onSaved={h.refreshManga}
              />
            )}

            {manga?.author && (
              <div className="rounded-lg bg-card border border-border p-4">
                <p className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground mb-1.5">
                  Author
                </p>
                <p className="text-sm font-medium">{manga.author}</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
