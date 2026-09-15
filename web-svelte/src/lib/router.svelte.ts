class Router {
  path = $state(window.location.pathname)

  constructor() {
    window.addEventListener('popstate', () => {
      this.path = window.location.pathname
    })
  }

  navigate(to: string, opts?: { replace?: boolean }) {
    if (opts?.replace) window.history.replaceState({}, '', to)
    else window.history.pushState({}, '', to)
    this.path = to
  }
}

export const router = new Router()

/** Matches a `/foo/:id` style pattern against a path, returning captured params or null. */
export function matchPath(pattern: string, path: string): Record<string, string> | null {
  const patternParts = pattern.split('/').filter(Boolean)
  const pathParts = path.split('/').filter(Boolean)
  if (patternParts.length !== pathParts.length) return null

  const params: Record<string, string> = {}
  for (let i = 0; i < patternParts.length; i++) {
    const part = patternParts[i]
    if (part.startsWith(':')) params[part.slice(1)] = decodeURIComponent(pathParts[i])
    else if (part !== pathParts[i]) return null
  }
  return params
}
