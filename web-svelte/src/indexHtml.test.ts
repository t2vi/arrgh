// spec: 001/FR-001a — app identity assets survive the port (GH #205)
import { describe, it, expect } from 'vitest'
import html from '../index.html?raw'

const publicFiles = Object.keys(import.meta.glob('../public/*', { query: '?url', eager: true })).map(
  (p) => p.replace('../public', ''),
)

describe('index.html identity assets', () => {
  it.each([
    ['/favicon.ico'],
    ['/favicon-32x32.png'],
    ['/favicon-16x16.png'],
    ['/apple-touch-icon.png'],
    ['/site.webmanifest'],
  ])('links %s and ships the file', (href) => {
    expect(html).toContain(`href="${href}"`)
    expect(publicFiles).toContain(href)
  })

  it('sets the Obsidian theme color', () => {
    expect(html).toContain('<meta name="theme-color" content="#121415" />')
  })

  it('ships no Vite scaffold icons', () => {
    expect(html).not.toContain('favicon.svg')
    expect(publicFiles).not.toContain('/favicon.svg')
    expect(publicFiles).not.toContain('/icons.svg')
  })
})
