import { useEffect, useRef } from 'react'
import { marked } from 'marked'

interface Props {
  content: string
  onRead: () => void
}

export function NovelReader({ content, onRead }: Props) {
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
      if (didReport.current) return
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 100) {
        didReport.current = true
        onRead()
      }
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  }, [onRead])

  return (
    <div ref={scrollRef} className="novel-scroll">
      <div className="mx-auto max-w-2xl px-6 py-10">
        <div
          className="novel-prose"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </div>
    </div>
  )
}
