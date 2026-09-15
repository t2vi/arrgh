import '@testing-library/jest-dom/vitest'
import { beforeEach } from 'vitest'
// 'allure-vitest/setup' in vite.config.ts's setupFiles provides the runtime
// `allure` global; its type comes from ./allure-vitest.d.ts (allure-vitest
// v3's package.json "exports" map has no "." entry to import it from).

beforeEach(async () => {
  await allure.layer('UI')
  await allure.tag('Web')
})
