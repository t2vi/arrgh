import { setContext } from "./manhuafast"
import type { PluginContext } from "./manhuafast"
import * as m from './manhuafast'

export const info = {
  id: 'manhuafast',
  name: 'ManhuaFast',
  default_explicit: false,
  content_types: ['manhua'],
}

export function init(ctx: PluginContext): void { setContext(ctx) }

export const search   = m.search
export const chapters = m.chapters
export const pages    = m.pages
