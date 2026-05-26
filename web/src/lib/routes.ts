export const ROUTES = {
  home:     '/',
  library:  '/library',
  discover: '/discover',
  queue:    '/queue',
  settings: '/settings',
  login:    '/login',
  setup:    '/setup',
  title:    (id: string) => `/title/${id}`,
  reader:   (id: string) => `/reader/${id}`,
}
