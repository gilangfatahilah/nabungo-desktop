import { getOnboardingStatus, securityStatus } from '@/ipc'
import { createRouter, createWebHashHistory, type NavigationGuard, type RouteRecordRaw } from 'vue-router'

export type RouteName =
  | 'dashboard'
  | 'transactions'
  | 'budget'
  | 'allocation'
  | 'reports'
  | 'accounts'
  | 'categories'
  | 'scheduled'
  | 'settings'
  | 'onboarding'
  | 'unlock'

type NamedRoute = RouteRecordRaw & { name: RouteName }

export const routes: NamedRoute[] = [
  { path: '/', name: 'dashboard', component: () => import('@/views/DashboardView.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('@/views/TransactionsView.vue'), meta: { wide: true } },
  { path: '/budget', name: 'budget', component: () => import('@/views/BudgetView.vue') },
  { path: '/allocation', name: 'allocation', component: () => import('@/views/AllocationView.vue') },
  { path: '/reports', name: 'reports', component: () => import('@/views/ReportsView.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('@/views/AccountsView.vue') },
  { path: '/categories', name: 'categories', component: () => import('@/views/CategoriesView.vue') },
  { path: '/scheduled', name: 'scheduled', component: () => import('@/views/ScheduledView.vue') },
  { path: '/settings', name: 'settings', component: () => import('@/views/SettingsView.vue') },
  // `bare`: tanpa sidebar, palette, sheet, dan shortcut global.
  { path: '/onboarding', name: 'onboarding', component: () => import('@/views/OnboardingView.vue'), meta: { bare: true } },
  { path: '/unlock', name: 'unlock', component: () => import('@/views/UnlockView.vue'), meta: { bare: true } },
]

declare module 'vue-router' {
  interface RouteMeta {
    bare?: boolean
    /** Konten melebar penuh, tanpa max-width (docs/design/ui-guidelines.md §7.1). */
    wide?: boolean
  }
}

/** Terkunci → semua route ke `unlock`; tidak terkunci → `unlock` dialihkan ke dasbor. */
export function securityGuard(isLocked: () => Promise<boolean>): NavigationGuard {
  return async (to) => {
    const locked = await isLocked()
    if (locked && to.name !== 'unlock') return { name: 'unlock' }
    if (!locked && to.name === 'unlock') return { name: 'dashboard' }
    return true
  }
}

/**
 * Belum onboarding → semua route ke `onboarding`; sudah → `onboarding` dialihkan ke dasbor.
 * `unlock` dilewatkan: saat terkunci status onboarding tidak bisa dibaca.
 */
export function onboardingGuard(isOnboarded: () => Promise<boolean>): NavigationGuard {
  return async (to) => {
    if (to.name === 'unlock') return true
    const done = await isOnboarded()
    if (!done && to.name !== 'onboarding') return { name: 'onboarding' }
    if (done && to.name === 'onboarding') return { name: 'dashboard' }
    return true
  }
}

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

// Dicek tiap navigasi: lock bisa terjadi kapan saja (idle, perintah backend).
router.beforeEach(securityGuard(async () => (await securityStatus()).locked))

// Status hanya di-cache setelah `true`: onboarding tidak bisa dibatalkan.
let onboarded = false
router.beforeEach(
  onboardingGuard(async () => {
    if (!onboarded) onboarded = await getOnboardingStatus()
    return onboarded
  }),
)
