import 'dotenv/config'
import express from 'express'
import * as rr from './royalroad'

const PORT = parseInt(process.env.PORT ?? '4004', 10)
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
    id: 'royalroad',
    name: 'Royal Road',
    default_explicit: false,
    content_types: ['novel'],
  })
})

app.get('/search', async (req, res) => {
  if (!auth(req, res)) return
  const q = String(req.query['q'] ?? '').trim()
  if (!q) return void res.json([])
  try {
    res.json(await rr.search(q))
  } catch (e) {
    console.error('search error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:id/chapters', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await rr.chapters(req.params['id']!))
  } catch (e) {
    console.error('chapters error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/chapter/:id/text', async (req, res) => {
  if (!auth(req, res)) return
  try {
    const text = await rr.chapterText(req.params['id']!)
    res.type('text/plain').send(text)
  } catch (e) {
    console.error('chapter text error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.get('/manga/:id/meta', async (req, res) => {
  if (!auth(req, res)) return
  try {
    res.json(await rr.meta(req.params['id']!))
  } catch (e) {
    console.error('meta error:', e)
    res.status(502).json({ error: String(e) })
  }
})

app.listen(PORT, () => {
  console.log(`[royalroad-plugin] listening on :${PORT}`)
  console.log(`  auth: ${API_KEY ? 'enabled' : 'disabled (no API_KEY set)'}`)
})
