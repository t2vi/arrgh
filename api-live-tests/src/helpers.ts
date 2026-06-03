const BASE = process.env.API_URL ?? 'http://localhost:3000'

export async function getToken(): Promise<string> {
  if (process.env.API_TOKEN) return process.env.API_TOKEN

  const user = process.env.API_USER ?? 'admin'
  const pass = process.env.API_PASS
  if (!pass) throw new Error('Set API_PASS (or API_TOKEN) to run API live tests')

  const res = await fetch(`${BASE}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: user, password: pass }),
  })
  if (!res.ok) throw new Error(`Login failed: ${res.status} ${await res.text()}`)
  const { token } = await res.json() as { token: string }
  return token
}

export async function apiGet<T>(path: string, token: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  if (!res.ok) throw new Error(`GET ${path} → ${res.status} ${await res.text()}`)
  return res.json() as Promise<T>
}
