import { useState } from 'react'
import { api, getUsername } from '@/api'
import type { Manga } from '@/types'

function greeting(): string {
  const hour = new Date().getHours()
  if (hour < 12) return 'Good morning'
  if (hour < 17) return 'Good afternoon'
  return 'Good evening'
}

export function GreetingJumbotron({
  totalManga,
  totalRead,
  coverManga,
}: {
  totalManga: number
  totalRead: number
  coverManga: Pick<Manga, 'id' | 'cover_url'> | null
}) {
  const [failed, setFailed] = useState(false)
  const username = getUsername()
  const coverSrc = !failed && coverManga
    ? (coverManga.cover_url?.startsWith('http') ? coverManga.cover_url : api.coverUrl(coverManga.id))
    : null

  return (
    <div className="relative overflow-hidden" style={{ minHeight: 172 }}>
      {coverSrc && (
        <img
          src={coverSrc}
          alt=""
          className="absolute inset-0 w-full h-full object-cover object-top scale-110"
          style={{ filter: 'blur(32px)', opacity: 0.18 }}
          onError={() => setFailed(true)}
        />
      )}
      <div className="absolute inset-0 bg-gradient-to-b from-background/60 via-background/80 to-background" />
      <div className="absolute inset-0 bg-gradient-to-r from-primary/5 to-transparent" />
      <div className="relative z-10 px-8 pt-10 pb-8">
        <p className="text-[11px] font-bold uppercase tracking-[0.25em] text-primary mb-2">*ARRgh</p>
        <h1 className="text-4xl font-extrabold tracking-tight leading-none mb-3">
          {greeting()}{username ? `, ${username}` : ''}.
        </h1>
        <div className="flex items-center gap-3 text-sm text-muted-foreground flex-wrap">
          {totalManga > 0 && (
            <span className="flex items-center gap-1.5">
              <span className="w-1 h-1 rounded-full bg-primary inline-block" />
              <span><span className="font-semibold text-foreground">{totalManga}</span> manga in library</span>
            </span>
          )}
          {totalRead > 0 && (
            <span className="flex items-center gap-1.5">
              <span className="w-1 h-1 rounded-full bg-muted-foreground inline-block" />
              <span><span className="font-semibold text-foreground">{totalRead}</span> chapters read</span>
            </span>
          )}
          {totalManga === 0 && (
            <span className="text-muted-foreground">Your library is empty — discover some manga to get started.</span>
          )}
        </div>
      </div>
    </div>
  )
}
