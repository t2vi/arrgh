import * as a from './asurascans'

export const info = {
  id: 'asurascans',
  name: 'AsuraScans',
  default_explicit: false,
  content_types: ['manhwa'],
}

export function init(_ctx: unknown): void {
  // Direct fetch — no browser needed
}

export const search   = a.search
export const chapters = a.chapters
export const pages    = a.pages
