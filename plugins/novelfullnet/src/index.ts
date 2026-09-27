import { setContext } from './novelfullnet'
import * as nfn from './novelfullnet'
import type { PluginContext } from './novelfullnet'

export const info = {
  id: 'novelfullnet',
  version: '1.0.0',
  name: 'NovelFull.net',
  default_explicit: false,
  content_types: ['novel'],
}

export function init(ctx: PluginContext): void {
  setContext(ctx)
}

export const search = nfn.search
export const meta = nfn.meta
export const chapterText = nfn.chapterText
export const parseSearchHtml = nfn.parseSearchHtml
export const parseChapterList = nfn.parseChapterList
export const parseChapterText = nfn.parseChapterText
export const parseMeta = nfn.parseMeta

export function chapters(id: string, _langs?: string[]): Promise<nfn.ChapterResult[]> {
  return nfn.chapters(id)
}
