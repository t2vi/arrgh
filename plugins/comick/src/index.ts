import { setContext } from './comick'
import * as c from './comick'
import type { PluginContext } from './comick'

const LANGS = (process.env.LANGUAGES ?? 'en').split(',').map((s) => s.trim()).filter(Boolean)

export const info = {
  id: 'comick',
  name: 'Comick',
  default_explicit: false,
  content_types: ['manga', 'manhwa', 'manhua'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search = c.search
export const trending = c.trending
export const meta = c.meta
export const pages = c.pages
export const cover = c.cover

export function chapters(id: string, langs?: string[]): Promise<c.ChapterItem[]> {
  return c.chapters(id, langs ?? LANGS)
}
