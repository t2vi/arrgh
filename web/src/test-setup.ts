import '@testing-library/jest-dom'
import 'allure-vitest'

beforeEach(async () => {
  await allure.layer('UI')
  await allure.tag('Web')
})
