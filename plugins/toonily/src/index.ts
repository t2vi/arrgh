import 'dotenv/config'
import express from 'express'
import * as t from './toonily'

const PORT = parseInt(process.env.PORT ?? '4001', 10)
const API_KEY = process.env.API_KEY ?? ''

const app = express()

function auth(req: express.Request, res: express.Response): boolean {
  if (!API_KEY) return true
  const header = req.headers.authorization ?? ''
  if (header !== `Bearer ${API_KEY}`) {
    res.status(401).json({ error: 'unauthorized' })
    return false
  }
  return true
}

app.get('/info', (req, res) => {
  if (!auth(req, res)) return
  res.json({
    id: 'toonily',
    name: 'Toonily',
    default_explicit: false,
    content_types: ['manhwa'],
  })
})

app.get('/search', async (req, res) => {
  if (!auth(req, res)) return
  const q = String(req.query['q'] ?? '').trim()
  if (!q) return void res.json([])
  try {
    res.json(await t.search(q))
  } catch (e) {
    console.error('search error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/trending', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await t.trending())
  } catch (e) {
    console.error('trending error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:slug/meta', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await t.meta(req.params['slug']!))
  } catch (e) {
    console.error('meta error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:slug/chapters', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await t.chapters(req.params['slug']!))
  } catch (e) {
    console.error('chapters error:', e)
    res.status(502).json({ error: String(e) })
  }
})

// source_id contains a slash: {manga_slug}/{chapter_slug}
// URL: /chapter/tower-of-god/chapter-578-0/pages
app.get(/^\/chapter\/(.+)\/pages$/, async (req, res) => {
  if (!auth(req, res)) return
  const sourceId = (req.params as unknown as string[])[0]!
  try {
    res.json(await t.pages(sourceId))
  } catch (e) {
    console.error('pages error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.listen(PORT, () => {
  console.log(`[toonily-plugin] listening on :${PORT}`)
  console.log(`  FlareSolverr: ${process.env.FLARESOLVERR_URL ?? 'http://flaresolverr:8191'}`)
  console.log(`  auth: ${API_KEY ? 'enabled' : 'disabled (no API_KEY set)'}`)
})
