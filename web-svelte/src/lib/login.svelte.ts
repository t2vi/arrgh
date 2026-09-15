import { router } from './router.svelte'
import { api, setToken } from './api'
import { ROUTES } from './routes'

export class LoginStore {
  username = $state('')
  password = $state('')
  error = $state('')
  loading = $state(false)

  async submit(e: SubmitEvent) {
    e.preventDefault()
    this.error = ''
    this.loading = true
    try {
      const res = await api.login(this.username.trim(), this.password)
      setToken(res.token, res.username, res.role, res.allow_explicit)
      router.navigate(ROUTES.home, { replace: true })
    } catch {
      this.error = 'Invalid username or password.'
    } finally {
      this.loading = false
    }
  }
}
