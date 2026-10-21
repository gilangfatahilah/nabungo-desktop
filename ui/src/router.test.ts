import { describe, expect, it, vi } from 'vitest'
import type { RouteLocationNormalized } from 'vue-router'
import { onboardingGuard, routes, securityGuard, type RouteName } from '@/router'
import id from '@/i18n/id'
import en from '@/i18n/en'

const NAMES: RouteName[] = [
  'dashboard', 'transactions', 'budget', 'allocation', 'reports',
  'accounts', 'categories', 'scheduled', 'settings', 'onboarding', 'unlock',
]

describe('router', () => {
  it('punya sebelas route bernama', () => {
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

  it('terkunci → semua route ke unlock', async () => {
    const g = securityGuard(async () => true)
    for (const name of NAMES.filter((n) => n !== 'unlock')) {
      expect(await run(g, name)).toEqual({ name: 'unlock' })
    }
    expect(await run(g, 'unlock')).toBe(true)
  })

  it('tidak terkunci → unlock ke dashboard', async () => {
    const g = securityGuard(async () => false)
    expect(await run(g, 'unlock')).toEqual({ name: 'dashboard' })
    expect(await run(g, 'accounts')).toBe(true)
  })

  it('terkunci mendahului onboarding', async () => {
    // Saat terkunci status onboarding tidak bisa dibaca; guard onboarding melewatkan `unlock`.
    const isOnboarded = vi.fn(async () => false)
    expect(await run(securityGuard(async () => true), 'onboarding')).toEqual({ name: 'unlock' })
    expect(await run(onboardingGuard(isOnboarded), 'unlock')).toBe(true)
    expect(isOnboarded).not.toHaveBeenCalled()
  })
})

describe('route meta', () => {
  it('hanya Transaksi yang melebar penuh (tanpa max-width konten)', () => {
    expect(routes.filter((r) => r.meta?.wide).map((r) => r.name)).toEqual(['transactions'])
  })
})
