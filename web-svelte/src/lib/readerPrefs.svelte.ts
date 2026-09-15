const FONT_SIZES = [14, 16, 18, 21] as const
const FONT_STORAGE_KEY = 'reader-font-size'

export function createNovelFontSize() {
  const stored = localStorage.getItem(FONT_STORAGE_KEY)
  const parsed = stored ? parseInt(stored, 10) : NaN
  let size = $state(FONT_SIZES.includes(parsed as (typeof FONT_SIZES)[number]) ? parsed : 16)

  return {
    get size() {
      return size
    },
    apply(s: number) {
      size = s
      localStorage.setItem(FONT_STORAGE_KEY, String(s))
    },
  }
}

const ZOOM_LEVELS = [50, 75, 100, 125, 150] as const
const ZOOM_STORAGE_KEY = 'reader-image-zoom'

export function createImageZoom() {
  const stored = localStorage.getItem(ZOOM_STORAGE_KEY)
  const parsed = stored ? parseInt(stored, 10) : NaN
  let zoom = $state((ZOOM_LEVELS as readonly number[]).includes(parsed) ? parsed : 100)

  return {
    get zoom() {
      return zoom
    },
    apply(z: number) {
      zoom = z
      localStorage.setItem(ZOOM_STORAGE_KEY, String(z))
    },
  }
}
