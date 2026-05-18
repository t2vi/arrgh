import express from 'express'
import * as md from './mangadex'

const PORT = parseInt(process.env.PORT ?? '4001', 10)
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
    id: 'mangadex',
    name: 'MangaDex',
    default_explicit: false,
    content_types: ['manga', 'manhwa', 'manhua', 'one-shot'],
  })
})

app.get('/search', async (req, res) => {
  if (!auth(req, res)) return
  const q = String(req.query['q'] ?? '').trim()
  if (!q) return void res.json([])
  try {
    res.json(await md.search(q))
  } catch (e) {
    console.error('search error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/trending', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await md.trending())
  } catch (e) {
    console.error('trending error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:id/meta', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await md.meta(req.params['id']!, LANGS))
  } catch (e) {
    console.error('meta error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:id/chapters', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await md.chapters(req.params['id']!, LANGS))
  } catch (e) {
    console.error('chapters error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/chapter/:id/pages', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await md.pages(req.params['id']!))
  } catch (e) {
    console.error('pages error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.listen(PORT, () => {
  console.log(`[mangadex-plugin] listening on :${PORT}`)
  console.log(`  languages: ${LANGS.join(', ')}`)
  console.log(`  auth: ${API_KEY ? 'enabled' : 'disabled (no API_KEY set)'}`)
})
