import { search, chapters, pages, setContext } from './manga18fx'
import type { PluginContext } from './manga18fx'

export function init(ctx: PluginContext): void { setContext(ctx) }

export const info = {
  id: 'manga18fx',
  name: 'Manga18fx',
  default_explicit: true,
  content_types: ['manhwa'],
  is_community: false,
}

export { search, chapters, pages }
