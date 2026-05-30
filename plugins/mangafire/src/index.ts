import * as mf from './mangafire'

export const info = {
  id: 'mangafire',
  name: 'MangaFire',
  default_explicit: false,
  content_types: ['manga', 'manhwa', 'manhua', 'one-shot'],
}

export function init(_ctx: unknown): void {
  // Direct fetch — no browser needed
}

export const search   = mf.search
export const chapters = mf.chapters
export const pages    = mf.pages
