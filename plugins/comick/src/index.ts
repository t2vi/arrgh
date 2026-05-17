import 'dotenv/config'
import express from 'express'
import * as c from './comick'

const PORT = parseInt(process.env.PORT ?? '4002', 10)
const API_KEY = process.env.API_KEY ?? ''
const LANGS = (process.env.LANGUAGES ?? 'en').split(',').map((s) => s.trim()).filter(Boolean)

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
    id: 'comick',
    name: 'Comick',
    default_explicit: false,
    content_types: ['manga', 'manhwa', 'manhua'],
  })
})

app.get('/search', async (req, res) => {
  if (!auth(req, res)) return
  const q = String(req.query['q'] ?? '').trim()
  if (!q) return void res.json([])
  try {
    res.json(await c.search(q))
  } catch (e) {
    console.error('search error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/trending', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await c.trending())
  } catch (e) {
    console.error('trending error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:slug/meta', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await c.meta(req.params['slug']!))
  } catch (e) {
    console.error('meta error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:slug/chapters', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await c.chapters(req.params['slug']!, LANGS))
  } catch (e) {
    console.error('chapters error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/chapter/:hid/pages', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await c.pages(req.params['hid']!))
  } catch (e) {
    console.error('pages error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.listen(PORT, () => {
  console.log(`[comick-plugin] listening on :${PORT}`)
  console.log(`  languages: ${LANGS.join(', ')}`)
  console.log(`  auth: ${API_KEY ? 'enabled' : 'disabled (no API_KEY set)'}`)
})
