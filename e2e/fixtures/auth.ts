import { test as base, expect } from '@playwright/test'

export { expect }

export const test = base.extend({
  // Override storageState per-test to use the seeded admin session
  storageState: '.auth/admin.json',
})
