import { useEffect } from 'react'

type Dir = 'ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight'

function getNavEls(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>('[data-nav]')).filter((el) => {
    const r = el.getBoundingClientRect()
    return r.width > 0 && r.height > 0
  })
}

function center(r: DOMRect) {
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 }
}

function spatialNavigate(dir: Dir) {
  const els = getNavEls()
  if (!els.length) return

  const active = document.activeElement as HTMLElement | null
  const fromEl = active && els.includes(active) ? active : null

  if (!fromEl) {
    els[0].focus()
    return
  }

  const from = center(fromEl.getBoundingClientRect())

  const candidates = els.filter((el) => {
    if (el === fromEl) return false
    const c = center(el.getBoundingClientRect())
    switch (dir) {
      case 'ArrowRight': return c.x > from.x + 4
      case 'ArrowLeft':  return c.x < from.x - 4
      case 'ArrowDown':  return c.y > from.y + 4
      case 'ArrowUp':    return c.y < from.y - 4
    }
  })

  if (!candidates.length) return

  const scored = candidates.map((el) => {
    const c = center(el.getBoundingClientRect())
    const dx = c.x - from.x
    const dy = c.y - from.y
    let primary: number
    let perp: number
    if (dir === 'ArrowRight' || dir === 'ArrowLeft') {
      primary = Math.abs(dx)
      perp = Math.abs(dy)
    } else {
      primary = Math.abs(dy)
      perp = Math.abs(dx)
    }
    return { el, score: primary + perp * 2 }
  })

  scored.sort((a, b) => a.score - b.score)
  scored[0].el.focus()
}

export function useDpadNav() {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (!['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) return
      const tag = (document.activeElement as HTMLElement)?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return
      e.preventDefault()
      spatialNavigate(e.key as Dir)
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])
}
