import { describe, expect, it } from 'vitest'
import type { RouteLocationNormalized } from 'vue-router'
import { onboardingGuard, routes, type RouteName } from '@/router'
import id from '@/i18n/id'
import en from '@/i18n/en'

const NAMES: RouteName[] = [
  'dashboard', 'transactions', 'budget', 'allocation', 'reports',
  'accounts', 'categories', 'scheduled', 'settings', 'onboarding',
]

describe('router', () => {
  it('punya sepuluh route bernama', () => {
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

  const to = (name: RouteName) => ({ name }) as unknown as RouteLocationNormalized
  const from = to('dashboard')
  // Guard memakai signature (to, from); argumen ketiga (next) tidak dipakai.
  const run = (g: ReturnType<typeof onboardingGuard>, name: RouteName) =>
    (g as (t: RouteLocationNormalized, f: RouteLocationNormalized) => unknown)(to(name), from)

  it('belum onboarding → semua route diarahkan ke onboarding', async () => {
    const g = onboardingGuard(async () => false)
    expect(await run(g, 'accounts')).toEqual({ name: 'onboarding' })
    expect(await run(g, 'onboarding')).toBe(true)
  })

  it('sudah onboarding → onboarding diarahkan ke dashboard, route lain dibiarkan', async () => {
    const g = onboardingGuard(async () => true)
    expect(await run(g, 'onboarding')).toEqual({ name: 'dashboard' })
    expect(await run(g, 'accounts')).toBe(true)
  })
})
