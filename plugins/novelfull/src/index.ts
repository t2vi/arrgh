import { setContext } from './novelfull'
import * as nf from './novelfull'
import type { PluginContext } from './novelfull'

export const info = {
  id: 'novelfull',
  name: 'NovelFull',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search = nf.search
export const meta = nf.meta
export const chapterText = nf.chapterText

export function chapters(id: string, _langs?: string[]): Promise<nf.ChapterResult[]> {
  return nf.chapters(id)
}
