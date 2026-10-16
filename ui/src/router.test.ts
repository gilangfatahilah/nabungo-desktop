import { describe, expect, it } from 'vitest'
import { routes, type RouteName } from '@/router'
import id from '@/i18n/id'
import en from '@/i18n/en'

const NAMES: RouteName[] = [
  'dashboard', 'transactions', 'budget', 'allocation', 'reports',
  'accounts', 'categories', 'scheduled', 'settings',
]

describe('router', () => {
  it('punya sembilan route bernama', () => {
    expect(routes.map((r) => r.name)).toEqual(NAMES)
  })

  it('path = /<nama> (dashboard = /)', () => {
    for (const r of routes) {
      expect(r.path).toBe(r.name === 'dashboard' ? '/' : `/${String(r.name)}`)
    }
  })

  it.each(NAMES)('label nav %s ada di id dan en', (name) => {
    expect(id.nav[name]).toBeTruthy()
    expect(en.nav[name]).toBeTruthy()
  })
})
