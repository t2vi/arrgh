import { setContext } from './wuxiaworld'
import * as w from './wuxiaworld'
import type { PluginContext } from './wuxiaworld'

export const info = {
  id: 'wuxiaworld',
  name: 'WuxiaWorld',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search      = w.search
export const chapters    = w.chapters
export const chapterText = w.chapterText
