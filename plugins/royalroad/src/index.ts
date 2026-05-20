import * as rr from './royalroad'

export const info = {
  id: 'royalroad',
  name: 'Royal Road',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(_ctx: unknown): void {
  // Direct fetch — no browser needed
}

export const search = rr.search
export const meta = rr.meta
export const chapterText = rr.chapterText

export function chapters(id: string, _langs?: string[]): Promise<rr.ChapterResult[]> {
  return rr.chapters(id)
}
