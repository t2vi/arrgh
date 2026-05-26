import { useEffect, useRef } from 'react'
import { marked } from 'marked'

interface Props {
  content: string
  fontSize: number
  onRead: () => void
  onProgress: (pct: number) => void
}

export function NovelReader({ content, fontSize, onRead, onProgress }: Props) {
  const didReport = useRef(false)
  const scrollRef = useRef<HTMLDivElement>(null)
  const html = marked.parse(content) as string

  useEffect(() => {
    didReport.current = false
  }, [content])

  useEffect(() => {
    const el = scrollRef.current
    if (!el) return
    const onScroll = () => {
      const pct = el.scrollHeight <= el.clientHeight
        ? 1
        : el.scrollTop / (el.scrollHeight - el.clientHeight)
      onProgress(Math.min(1, pct))
      if (didReport.current) return
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 100) {
        didReport.current = true
        onRead()
      }
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  }, [onRead, onProgress])

  return (
    <div ref={scrollRef} className="novel-scroll">
      <div className="mx-auto max-w-2xl px-6 py-10">
        <div
          className="novel-prose"
          style={{ fontSize }}
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </div>
    </div>
  )
}
