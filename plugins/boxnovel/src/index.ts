import * as b from './boxnovel'

export const info = {
  id: 'boxnovel',
  name: 'BoxNovel',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(_ctx: unknown): void {
  // Direct fetch — no browser needed
}

export const search      = b.search
export const chapters    = b.chapters
export const chapterText = b.chapterText
