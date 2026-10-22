<script setup lang="ts">
import { computed, onMounted } from 'vue'
import {
  CalendarClock,
  ChartColumn,
  ChartPie,
  Landmark,
  LayoutDashboard,
  Plus,
  ReceiptText,
  Search,
  Settings,
  Shapes,
  Tags,
  Wallet,
} from '@lucide/vue'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar'
import { Button } from '@/components/ui/button'
import { Kbd } from '@/components/ui/kbd'
import AppLogo from '@/components/AppLogo.vue'
import NavLinks, { type NavItem } from '@/components/layout/NavLinks.vue'
import NavSummary from '@/components/layout/NavSummary.vue'
import { withErrorToast } from '@/lib/notify'
import { useScheduledStore } from '@/stores/scheduled'
import { useUiStore } from '@/stores/ui'

// Struktur mengikuti shadcn-vue dashboard-01 (docs/design/design.md §6.2).
const main: NavItem[] = [
  { name: 'dashboard', to: '/', icon: LayoutDashboard, keys: 'G D' },
  { name: 'transactions', to: '/transactions', icon: ReceiptText, keys: 'G T' },
  { name: 'budget', to: '/budget', icon: Wallet, keys: 'G B' },
  { name: 'allocation', to: '/allocation', icon: ChartPie, keys: 'G A' },
  { name: 'reports', to: '/reports', icon: ChartColumn, keys: 'G R' },
]
const secondary: NavItem[] = [{ name: 'settings', to: '/settings', icon: Settings, keys: 'Ctrl+,' }]

const ui = useUiStore()
const scheduled = useScheduledStore()
// Memuat jadwal sekaligus menjalankan auto-post saat app dibuka.
onMounted(() => void withErrorToast(() => scheduled.reload()))

const manage = computed<NavItem[]>(() => [
  { name: 'accounts', to: '/accounts', icon: Landmark },
  { name: 'categories', to: '/categories', icon: Shapes },
  { name: 'labels', to: '/labels', icon: Tags },
  { name: 'scheduled', to: '/scheduled', icon: CalendarClock, badge: scheduled.dueCount },
])
</script>

<template>
  <Sidebar variant="inset" collapsible="icon">
    <SidebarHeader>
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton as-child class="data-[slot=sidebar-menu-button]:p-1.5!">
            <RouterLink to="/"><AppLogo /></RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarHeader>
    <SidebarContent>
      <!-- Quick create: satu-satunya blok Zamrud di sidebar. -->
      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem class="flex items-center gap-2">
              <SidebarMenuButton
                :tooltip="$t('sidebar.newTransaction')"
                class="min-w-8 bg-primary text-primary-foreground duration-200 ease-linear hover:bg-primary/90 hover:text-primary-foreground active:bg-primary/90 active:text-primary-foreground"
                data-testid="new-txn"
                @click="ui.openNewTxn()"
              >
                <Plus />
                <span>{{ $t('sidebar.newTransaction') }}</span>
                <Kbd class="ml-auto border-current/35 text-current/75">N</Kbd>
              </SidebarMenuButton>
              <Button
                size="icon"
                variant="outline"
                class="size-8 shrink-0 group-data-[collapsible=icon]:opacity-0"
                :aria-label="$t('sidebar.search')"
                :title="$t('sidebar.search')"
                @click="ui.paletteOpen = true"
              >
                <Search />
              </Button>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
      <NavLinks :items="main" :aria-label="$t('sidebar.main')" />
      <NavLinks :items="manage" :label="$t('sidebar.manage')" />
      <NavLinks :items="secondary" class="mt-auto" />
    </SidebarContent>
    <SidebarFooter>
      <NavSummary />
    </SidebarFooter>
  </Sidebar>
</template>
