// Astro's glob loader strips dots from IDs: v0.0.12.md → v0012
// This reconstructs the semver string for the current v0.0.X scheme.
export function idToVersion(id: string): string {
  const m = id.match(/^v(\d)(\d)(\d+)$/)
  if (!m) return id
  return `v${m[1]}.${m[2]}.${m[3]}`
}
