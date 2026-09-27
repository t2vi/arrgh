import { setContext } from './novelupdates'
import * as nu from './novelupdates'
import type { PluginContext } from './novelupdates'

export const info = {
  id: 'novelupdates',
  version: '1.0.0',
  name: 'NovelUpdates',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search   = nu.search
export const chapters = nu.chapters
