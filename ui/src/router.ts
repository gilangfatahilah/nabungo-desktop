import { getOnboardingStatus } from '@/ipc'
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
]

declare module 'vue-router' {
  interface RouteMeta {
    bare?: boolean
    /** Konten melebar penuh, tanpa max-width (docs/design/ui-guidelines.md §7.1). */
    wide?: boolean
  }
}

/** Belum onboarding → semua route ke `onboarding`; sudah → `onboarding` dialihkan ke dasbor. */
export function onboardingGuard(isOnboarded: () => Promise<boolean>): NavigationGuard {
  return async (to) => {
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

// Status hanya di-cache setelah `true`: onboarding tidak bisa dibatalkan.
let onboarded = false
router.beforeEach(
  onboardingGuard(async () => {
    if (!onboarded) onboarded = await getOnboardingStatus()
    return onboarded
  }),
)
