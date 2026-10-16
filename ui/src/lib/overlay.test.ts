import { describe, expect, it } from 'vitest'
import { shouldIgnoreAction } from './overlay'

describe('shouldIgnoreAction', () => {
  it('mengabaikan navigate dan palette saat sheet terbuka', () => {
    expect(shouldIgnoreAction({ kind: 'navigate', route: 'budget' }, true)).toBe(true)
    expect(shouldIgnoreAction({ kind: 'palette' }, true)).toBe(true)
  })
  it('newTxn tidak diabaikan', () => {
    expect(shouldIgnoreAction({ kind: 'newTxn' }, true)).toBe(false)
  })
  it('tidak mengabaikan apa pun saat sheet tertutup', () => {
    expect(shouldIgnoreAction({ kind: 'palette' }, false)).toBe(false)
  })
})
