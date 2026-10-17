<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { ChevronDown } from '@lucide/vue'
import MoneyText from '@/components/MoneyText.vue'
import { balanceTone, groupAccounts } from '@/lib/accounts'
import { useAccountsStore } from '@/stores/accounts'
import { useUiStore } from '@/stores/ui'
import type { RouteName } from '@/router'

interface Item {
  name: RouteName
  to: string
  keys: string
}

// Hint shortcut sesuai PRD §9; handler-nya di App.vue (lib/shortcuts).
const main: Item[] = [
  { name: 'dashboard', to: '/', keys: 'G D' },
  { name: 'transactions', to: '/transactions', keys: 'G T' },
  { name: 'budget', to: '/budget', keys: 'G B' },
  { name: 'allocation', to: '/allocation', keys: 'G A' },
  { name: 'reports', to: '/reports', keys: 'G R' },
]
const manage: Item[] = [
  { name: 'accounts', to: '/accounts', keys: '' },
  { name: 'categories', to: '/categories', keys: '' },
  { name: 'scheduled', to: '/scheduled', keys: '' },
]
const settings: Item = { name: 'settings', to: '/settings', keys: 'Ctrl+,' }

const route = useRoute()
const ui = useUiStore()
const current = computed(() => route.name)
const accounts = useAccountsStore()
const sections = computed(() => groupAccounts(accounts.overview, { archived: false }))
</script>

<template>
  <Sidebar>
    <SidebarHeader class="gap-3 p-3">
      <p class="px-1 text-lg font-semibold">{{ $t('app.name') }}</p>
      <Button class="w-full justify-between" data-testid="new-txn" @click="ui.openNewTxn()">
        <span>+ {{ $t('sidebar.newTransaction') }}</span>
        <kbd class="rounded border border-primary-foreground/40 px-1.5 text-xs font-sans">N</kbd>
      </Button>
    </SidebarHeader>
    <SidebarContent>
      <SidebarGroup>
        <SidebarGroupLabel>{{ $t('sidebar.main') }}</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem v-for="item in main" :key="item.name">
              <SidebarMenuButton as-child :is-active="current === item.name">
                <RouterLink :to="item.to">
                  <span>{{ $t(`nav.${item.name}`) }}</span>
                  <kbd class="ml-auto text-xs font-sans text-muted-foreground">{{ item.keys }}</kbd>
                </RouterLink>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
      <SidebarGroup data-testid="sidebar-accounts">
        <RouterLink to="/accounts" class="flex items-baseline justify-between rounded-md px-2 py-1 hover:bg-sidebar-accent">
          <span class="text-xs font-medium text-sidebar-foreground/70">{{ $t('accounts.netWorth') }}</span>
          <MoneyText :amount="accounts.overview.netWorth.net" class="text-sm font-semibold" data-testid="sidebar-net-worth" />
        </RouterLink>
        <Collapsible v-for="s in sections" :key="s.group?.id ?? 'none'" default-open class="group/coll">
          <CollapsibleTrigger class="flex w-full items-center gap-1 px-2 py-1 text-xs text-sidebar-foreground/70">
            <ChevronDown class="size-3 transition-transform group-data-[state=closed]/coll:-rotate-90" />
            <span class="flex-1 truncate text-left">{{ s.group?.name ?? $t('accounts.noGroup') }}</span>
            <MoneyText :amount="s.subtotal" />
          </CollapsibleTrigger>
          <CollapsibleContent>
            <SidebarMenu>
              <SidebarMenuItem v-for="a in s.accounts" :key="a.id">
                <SidebarMenuButton as-child size="sm">
                  <RouterLink to="/accounts">
                    <span class="truncate">{{ a.name }}</span>
                    <MoneyText :amount="a.balance" :kind="balanceTone(a)" class="ml-auto text-xs" />
                  </RouterLink>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </CollapsibleContent>
        </Collapsible>
      </SidebarGroup>
      <SidebarGroup>
        <SidebarGroupLabel>{{ $t('sidebar.manage') }}</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem v-for="item in manage" :key="item.name">
              <SidebarMenuButton as-child :is-active="current === item.name">
                <RouterLink :to="item.to">
                  <span>{{ $t(`nav.${item.name}`) }}</span>
                </RouterLink>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
    <SidebarFooter>
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton as-child :is-active="current === settings.name">
            <RouterLink :to="settings.to">
              <span>{{ $t(`nav.${settings.name}`) }}</span>
              <kbd class="ml-auto text-xs font-sans text-muted-foreground">{{ settings.keys }}</kbd>
            </RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarFooter>
  </Sidebar>
</template>
