import { setContext } from './nhentai'
import * as n from './nhentai'
import type { PluginContext } from './nhentai'

export const info = {
  id: 'nhentai',
  name: 'nhentai',
  default_explicit: true,
  content_types: ['manga'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search = n.search
export const chapters = n.chapters
export const pages = n.pages
