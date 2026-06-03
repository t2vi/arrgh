// Verify candidate titles against live novelfull
// URL: /search?keyword=encodeURIComponent(query)
// Usage: CLOAK_WS_URL=http://localhost:3000 node verify-novelfull.mjs
import { chromium } from 'playwright-core'

const BASE = 'https://novelfull.com'

const CANDIDATES = [
  // apostrophe
  "The King's Avatar",
  "Omniscient Reader's Viewpoint",
  "Dragon Prince Yuan",
  // (Novel) suffix
  "Omniscient Reader's Viewpoint (Novel)",
  // colon
  "Mushoku Tensei: Jobless Reincarnation",
  // numbers
  "108 Maidens of Destiny",
  // ? in title
  "Is It Wrong to Try to Pick Up Girls in a Dungeon?",
  // standard popular titles
  "Martial God Asura",
  "Against the Gods",
  "Coiling Dragon",
  "Overgeared",
  "The Legendary Mechanic",
  "Release That Witch",
  // not found sentinel candidates
  "Dungeon Defense",
  "There Is No Epic Loot Here Only Puns",
]

const endpointUrl = process.env.CLOAK_WS_URL
if (!endpointUrl) { console.error('CLOAK_WS_URL not set'); process.exit(1) }

const versionRes = await fetch(`${endpointUrl}/json/version`)
const { webSocketDebuggerUrl } = await versionRes.json()
const wsUrl = webSocketDebuggerUrl.replace(/^ws:\/\/[^/]+/, `ws://localhost:${new URL(endpointUrl).port || 80}`)
const browser = await chromium.connectOverCDP(wsUrl)
const sharedCtx = await browser.newContext()

async function getPage(url) {
  const page = await sharedCtx.newPage()
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 60000 })
  const html = await page.content()
  await page.close()
  return html
}

function extractSlugs(html) {
  const matches = [...html.matchAll(/href="\/([a-z0-9-]+\.html)"/g)]
  return [...new Set(matches.map(m => m[1].replace(/\.html$/, '')))].slice(0, 3)
}

const confirmed = [], notFound = []

for (const title of CANDIDATES) {
  const url = `${BASE}/search?keyword=${encodeURIComponent(title)}`
  try {
    const html = await getPage(url)
    const slugs = extractSlugs(html)
    if (slugs.length > 0) {
      console.log(`✓  ${title} → [${slugs.join(', ')}]`)
      confirmed.push(title)
    } else {
      console.log(`✗  ${title} → not found`)
      notFound.push(title)
    }
  } catch (e) {
    console.log(`!  ${title} → ERROR: ${e.message}`)
    notFound.push(title)
  }
  await new Promise(r => setTimeout(r, 1500))
}

await sharedCtx.close()

console.log('\n=== CONFIRMED ===')
confirmed.forEach(t => console.log(` "${t}",`))
console.log('\n=== NOT FOUND ===')
notFound.forEach(t => console.log(` "${t}",`))
