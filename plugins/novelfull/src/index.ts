import 'dotenv/config'
import express from 'express'
import * as nf from './novelfull'

const PORT = parseInt(process.env.PORT ?? '4005', 10)
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
    id: 'novelfull',
    name: 'NovelFull',
    default_explicit: false,
    content_types: ['novel'],
  })
})

app.get('/search', async (req, res) => {
  if (!auth(req, res)) return
  const q = String(req.query['q'] ?? '').trim()
  if (!q) return void res.json([])
  try {
    res.json(await nf.search(q))
  } catch (e) {
    console.error('search error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:slug/chapters', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await nf.chapters(req.params['slug']!))
  } catch (e) {
    console.error('chapters error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/chapter/:id/text', async (req, res) => {
  if (!auth(req, res)) return
  try {
    const text = await nf.chapterText(req.params['id']!)
    res.type('text/plain').send(text)
  } catch (e) {
    console.error('chapter text error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:slug/meta', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await nf.meta(req.params['slug']!))
  } catch (e) {
    console.error('meta error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.listen(PORT, () => {
  console.log(`[novelfull-plugin] listening on :${PORT}`)
  console.log(`  FlareSolverr: ${process.env.FLARESOLVERR_URL ?? 'http://flaresolverr:8191'}`)
  console.log(`  auth: ${API_KEY ? 'enabled' : 'disabled (no API_KEY set)'}`)
})
