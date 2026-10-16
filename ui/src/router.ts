import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

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

type NamedRoute = RouteRecordRaw & { name: RouteName }

export const routes: NamedRoute[] = [
  { path: '/', name: 'dashboard', component: () => import('@/views/DashboardView.vue') },
  { path: '/transactions', name: 'transactions', component: () => import('@/views/TransactionsView.vue') },
  { path: '/budget', name: 'budget', component: () => import('@/views/BudgetView.vue') },
  { path: '/allocation', name: 'allocation', component: () => import('@/views/AllocationView.vue') },
  { path: '/reports', name: 'reports', component: () => import('@/views/ReportsView.vue') },
  { path: '/accounts', name: 'accounts', component: () => import('@/views/AccountsView.vue') },
  { path: '/categories', name: 'categories', component: () => import('@/views/CategoriesView.vue') },
  { path: '/scheduled', name: 'scheduled', component: () => import('@/views/ScheduledView.vue') },
  { path: '/settings', name: 'settings', component: () => import('@/views/SettingsView.vue') },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})
