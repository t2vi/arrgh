import { describe, it, expect } from 'vitest'
import { cn } from './utils'
beforeEach(async () => {
  await allure.epic('Components')
  await allure.feature('Utils')
})

describe('cn', () => {
  it('merges class names', () => {
    expect(cn('a', 'b')).toBe('a b')
  })

  it('deduplicates tailwind classes', () => {
    expect(cn('p-2', 'p-4')).toBe('p-4')
  })
})
