import * as md from './mangadex'

const LANGS = (process.env.LANGUAGES ?? 'en').split(',').map((s) => s.trim()).filter(Boolean)

export const info = {
  id: 'mangadex',
  name: 'MangaDex',
  default_explicit: false,
  content_types: ['manga', 'manhwa', 'manhua', 'one-shot'],
}

export function init(_ctx: unknown): void {
  // MangaDex uses the public API — no browser needed
}

export const search = md.search
export const trending = md.trending
export const pages = md.pages

export function chapters(id: string, langs?: string[]): Promise<md.ChapterItem[]> {
  return md.chapters(id, langs ?? LANGS)
}

export function meta(id: string, langs?: string[]): Promise<md.MetaResponse> {
  return md.meta(id, langs ?? LANGS)
}
