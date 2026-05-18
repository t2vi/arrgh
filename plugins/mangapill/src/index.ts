import 'dotenv/config'
import express from 'express'
import * as m from './mangapill'

const PORT = parseInt(process.env.PORT ?? '4000', 10)
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
    id: 'mangapill',
    name: 'Mangapill',
    default_explicit: false,
    content_types: ['manga'],
  })
})

app.get('/search', async (req, res) => {
  if (!auth(req, res)) return
  const q = String(req.query['q'] ?? '').trim()
  if (!q) return void res.json([])
  try {
    res.json(await m.search(q))
  } catch (e) {
    console.error('search error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/trending', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await m.trending())
  } catch (e) {
    console.error('trending error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:id/meta', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await m.meta(req.params['id']!))
  } catch (e) {
    console.error('meta error:', e)
    res.status(502).json({ error: String(e) })
  }
})

// source_id contains a slash: {id-encoded}/{slug}
app.get(/^\/manga\/(.+)\/chapters$/, async (req, res) => {
  if (!auth(req, res)) return
  const sourceId = (req.params as unknown as string[])[0]!
  try {
    res.json(await m.chapters(sourceId))
  } catch (e) {
    console.error('chapters error:', e)
    res.status(502).json({ error: String(e) })
  }
})

// source_id contains a slash: {id-encoded}/{slug}
app.get(/^\/chapter\/(.+)\/pages$/, async (req, res) => {
  if (!auth(req, res)) return
  const sourceId = (req.params as unknown as string[])[0]!
  try {
    res.json(await m.pages(sourceId))
  } catch (e) {
    console.error('pages error:', e)
    res.status(502).json({ error: String(e) })
  }
})

// Fetch cover with correct Referer — used by arrgh for library cover downloads
app.get('/cover', async (req, res) => {
  if (!auth(req, res)) return
  const url = String(req.query['url'] ?? '').trim()
  if (!url) return void res.status(400).json({ error: 'url required' })
  try {
    const bytes = await m.fetchCoverBytes(url)
    const ct = url.includes('.webp') ? 'image/webp' : url.includes('.png') ? 'image/png' : 'image/jpeg'
    res.set('Content-Type', ct).send(bytes)
  } catch (e) {
    console.error('cover error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.listen(PORT, () => {
  console.log(`[mangapill-plugin] listening on :${PORT}`)
  console.log(`  auth: ${API_KEY ? 'enabled' : 'disabled (no API_KEY set)'}`)
})
