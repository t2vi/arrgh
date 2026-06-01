import { setContext } from "./boxnovel"
import type { PluginContext } from "./boxnovel"
import * as b from './boxnovel'

export const info = {
  id: 'boxnovel',
  name: 'BoxNovel',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(ctx: PluginContext): void { setContext(ctx) }

export const search      = b.search
export const chapters    = b.chapters
export const chapterText = b.chapterText
