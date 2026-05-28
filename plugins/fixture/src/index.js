import express from 'express'

const PORT = process.env.PORT ?? 4001

const INFO = {
  id: 'fixture',
  name: 'Fixture',
  default_explicit: false,
  content_types: ['manga'],
  is_community: false,
}

// Tiny 1×1 white JPEG — valid image for the downloader to fetch and write
const PIXEL_JPEG = Buffer.from(
  '/9j/4AAQSkZJRgABAQEASABIAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8U' +
  'HRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgN' +
  'DRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIy' +
  'MjIyMjL/wAARCAABAAEDASIAAhEBAxEB/8QAFgABAQEAAAAAAAAAAAAAAAABAgME/8QAFBAB' +
  'AAAAAAAAAAAAAAAAAAAAAP/EABQBAQAAAAAAAAAAAAAAAAAAAAD/xAAUEQEAAAAAAAAAAAAA' +
  'AAAAAAAA/9oADAMBAAIRAxEAPwCwABmX/9k=',
  'base64'
)

// ── Route helpers ─────────────────────────────────────────────────────────────

function isMode(id, mode) {
  return id.toLowerCase().includes(mode)
}

// ── App ───────────────────────────────────────────────────────────────────────

const app = express()

// Healthcheck
app.get('/health', (_req, res) => res.json({ ok: true }))

// Plugin list — what arrgh calls first to register sources
app.get('/plugins', (_req, res) => res.json([INFO]))

// Plugin info
app.get('/fixture/info', (_req, res) => res.json(INFO))

// Search — source matching calls this with the title name
// "Fixture No Match" → empty results (triggers Sync Warning)
// Everything else → return a matching result
app.get('/fixture/search', (req, res) => {
  const q = String(req.query.q ?? '').toLowerCase()
  if (q.includes('no match')) return res.json([])
  if (q.includes('502')) return res.json([{ id: 'fixture-502', title: q, cover_url: null, status: 'ongoing', content_type: 'manga' }])
  if (q.includes('empty pages')) return res.json([{ id: 'fixture-empty', title: q, cover_url: null, status: 'ongoing', content_type: 'manga' }])
  res.json([{ id: 'fixture-happy', title: q, cover_url: null, status: 'ongoing', content_type: 'manga' }])
})

// Chapters
// fixture-502   → returns HTTP 502 (source down → Err → sync_warning set)
// fixture-empty → returns 1 chapter; pages will return []
// everything else → returns 3 chapters
app.get('/fixture/manga/:id/chapters', (req, res) => {
  if (isMode(req.params.id, '502')) return res.status(502).json({ error: 'source unavailable' })
  if (isMode(req.params.id, 'empty')) {
    return res.json([
      { source_id: 'fixture-empty-c1', number: 1.0, title: 'Chapter 1', chapter_format: 'pages' },
    ])
  }
  res.json([
    { source_id: 'fixture-c1', number: 1.0, title: 'Chapter 1', chapter_format: 'pages' },
    { source_id: 'fixture-c2', number: 2.0, title: 'Chapter 2', chapter_format: 'pages' },
    { source_id: 'fixture-c3', number: 3.0, title: 'Chapter 3', chapter_format: 'pages' },
  ])
})

// Pages
// fixture-empty-* → return [] (triggers 0-pages guard)
// everything else → return 3 pages
app.get('/fixture/chapter/:id/pages', (req, res) => {
  if (isMode(req.params.id, 'empty')) return res.json([])
  const base = `http://fixture:${PORT}`
  res.json([`${base}/image.jpg`, `${base}/image.jpg`, `${base}/image.jpg`])
})

// Tiny JPEG served for downloader to actually fetch
app.get('/image.jpg', (_req, res) => {
  res.set('Content-Type', 'image/jpeg').send(PIXEL_JPEG)
})

app.listen(PORT, () => console.log(`[fixture] listening on :${PORT}`))
