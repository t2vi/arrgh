import * as m from './mangapill'

export const info = {
  id: 'mangapill',
  name: 'Mangapill',
  default_explicit: false,
  content_types: ['manga'],
}

export function init(_ctx: unknown): void {
  // Direct fetch — no browser needed
}

export const search = m.search
export const trending = m.trending
export const meta = m.meta
export const chapters = m.chapters
export const pages = m.pages

export async function cover(url: string): Promise<Buffer> {
  return m.fetchCoverBytes(url)
}
