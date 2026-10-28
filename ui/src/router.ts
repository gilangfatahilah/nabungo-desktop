import { getOnboardingStatus, recoveryStatus, securityStatus } from '@/ipc'
import { createRouter, createWebHashHistory, type NavigationGuard, type RouteRecordRaw } from 'vue-router'

export type RouteName =
  | 'dashboard'
  | 'transactions'
  | 'budget'
  | 'allocation'
  | 'reports'
  | 'accounts'
  | 'categories'
  | 'labels'
  | 'scheduled'
  | 'inbox'
  | 'settings'
  | 'onboarding'
  | 'unlock'
  | 'recovery'

type NamedRoute = RouteRecordRaw & { name: RouteName }

export const routes: NamedRoute[] = [
  { path: '/', name: 'dashboard', component: () => import('@/views/DashboardView.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('@/views/TransactionsView.vue') },
  { path: '/budget', name: 'budget', component: () => import('@/views/BudgetView.vue') },
  { path: '/allocation', name: 'allocation', component: () => import('@/views/AllocationView.vue') },
  { path: '/reports', name: 'reports', component: () => import('@/views/ReportsView.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('@/views/AccountsView.vue') },
  { path: '/categories', name: 'categories', component: () => import('@/views/CategoriesView.vue') },
  { path: '/labels', name: 'labels', component: () => import('@/views/LabelsView.vue') },
  { path: '/scheduled', name: 'scheduled', component: () => import('@/views/ScheduledView.vue') },
  { path: '/inbox', name: 'inbox', component: () => import('@/views/InboxView.vue') },
  { path: '/settings', name: 'settings', component: () => import('@/views/SettingsView.vue') },
  // `bare`: tanpa sidebar, palette, sheet, dan shortcut global.
  { path: '/onboarding', name: 'onboarding', component: () => import('@/views/OnboardingView.vue'), meta: { bare: true } },
  { path: '/unlock', name: 'unlock', component: () => import('@/views/UnlockView.vue'), meta: { bare: true } },
  { path: '/recovery', name: 'recovery', component: () => import('@/views/RecoveryView.vue'), meta: { bare: true } },
]

declare module 'vue-router' {
  interface RouteMeta {
    bare?: boolean
  }
}

/**
 * Pemulihan aktif (DB gagal dibuka) → semua route ke `recovery`. Tidak aktif → `recovery` hanya
 * boleh dibuka saat terkunci (tautan dari layar kunci); selain itu dialihkan ke dasbor.
 */
export function recoveryGuard(isRecovery: () => Promise<boolean>, isLocked: () => Promise<boolean>): NavigationGuard {
  return async (to) => {
    const active = await isRecovery()
    if (active) return to.name === 'recovery' ? true : { name: 'recovery' }
    if (to.name === 'recovery' && !(await isLocked())) return { name: 'dashboard' }
    return true
  }
}

/**
 * Terkunci → semua route ke `unlock`; tidak terkunci → `unlock` dialihkan ke dasbor.
 * `recovery` dilewatkan: diputuskan `recoveryGuard`, dan saat pemulihan `securityStatus` gagal.
 */
export function securityGuard(isLocked: () => Promise<boolean>): NavigationGuard {
  return async (to) => {
    if (to.name === 'recovery') return true
    const locked = await isLocked()
    if (locked && to.name !== 'unlock') return { name: 'unlock' }
    if (!locked && to.name === 'unlock') return { name: 'dashboard' }
    return true
  }
}

/**
 * Belum onboarding → semua route ke `onboarding`; sudah → `onboarding` dialihkan ke dasbor.
 * `unlock` dan `recovery` dilewatkan: status onboarding tidak bisa dibaca saat itu.
 */
export function onboardingGuard(isOnboarded: () => Promise<boolean>): NavigationGuard {
  return async (to) => {
    if (to.name === 'unlock' || to.name === 'recovery') return true
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

const isLocked = async () => (await securityStatus()).locked

// Sebelum guard keamanan: saat pemulihan `securityStatus` menjawab `recovery_required`.
router.beforeEach(recoveryGuard(async () => (await recoveryStatus()).active, isLocked))

// Dicek tiap navigasi: lock bisa terjadi kapan saja (idle, perintah backend).
router.beforeEach(securityGuard(isLocked))

// Status hanya di-cache setelah `true`: onboarding tidak bisa dibatalkan.
let onboarded = false
router.beforeEach(
  onboardingGuard(async () => {
    if (!onboarded) onboarded = await getOnboardingStatus()
    return onboarded
  }),
)
