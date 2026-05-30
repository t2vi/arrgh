import * as m from './manhuafast'

export const info = {
  id: 'manhuafast',
  name: 'ManhuaFast',
  default_explicit: false,
  content_types: ['manhua'],
}

export function init(_ctx: unknown): void {
  // Direct fetch — no browser needed
}

export const search   = m.search
export const chapters = m.chapters
export const pages    = m.pages
