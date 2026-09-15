import { router } from './router.svelte'
import { api, getToken } from './api'
import { ROUTES } from './routes'

export class SetupStore {
  step = $state<1 | 2>(1)

  constructor() {
    if (!getToken()) return
    // Already have a valid session — setup is done, redirect to library.
    api.me().then(() => router.navigate(ROUTES.home, { replace: true })).catch(() => {})
  }

  goToStep2() {
    this.step = 2
  }

  finish() {
    router.navigate(ROUTES.home, { replace: true })
  }
}
