export const ROUTES = {
  home:     '/',
  library:  '/library',
  discover: '/discover',
  queue:    '/queue',
  settings: '/settings',
  login:    '/login',
  setup:    '/setup',
  manga:    (id: string) => `/manga/${id}`,
  reader:   (id: string) => `/reader/${id}`,
}
