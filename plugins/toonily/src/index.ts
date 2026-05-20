import { setContext } from './toonily'
import * as t from './toonily'
import type { PluginContext } from './toonily'

export const info = {
  id: 'toonily',
  name: 'Toonily',
  default_explicit: false,
  content_types: ['manhwa'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search = t.search
export const trending = t.trending
export const meta = t.meta
export const chapters = t.chapters
export const pages = t.pages
