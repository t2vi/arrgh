import { setContext } from "./asurascans"
import type { PluginContext } from "./asurascans"
import * as a from './asurascans'

export const info = {
  id: 'asurascans',
  name: 'AsuraScans',
  default_explicit: false,
  content_types: ['manhwa'],
}

export function init(ctx: PluginContext): void { setContext(ctx) }

export const search   = a.search
export const chapters = a.chapters
export const pages    = a.pages
