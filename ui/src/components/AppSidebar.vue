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
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { ChevronDown, Plus } from '@lucide/vue'
import AppLogo from '@/components/AppLogo.vue'
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

// Item aktif: latar Lembar + garis Zamrud 2px di kiri (docs/design/ui-guidelines.md §7.4).
const itemClass = 'data-active:bg-background data-active:text-foreground data-active:shadow-[inset_2px_0_0_var(--primary)]'
// Petunjuk shortcut hanya saat hover/fokus atau item aktif.
const hintClass = 'ml-auto opacity-0 transition-opacity group-hover/menu-button:opacity-100 group-focus-visible/menu-button:opacity-100 group-data-active/menu-button:opacity-100'
const keys = (k: string) => k.split(/[ +]/)
</script>

<template>
  <Sidebar>
    <SidebarHeader class="gap-4 px-3 pt-5 pb-2">
      <AppLogo class="px-1" />
      <Button class="w-full justify-between" data-testid="new-txn" @click="ui.openNewTxn()">
        <span class="flex items-center gap-1.5"><Plus />{{ $t('sidebar.newTransaction') }}</span>
        <Kbd>N</Kbd>
      </Button>
    </SidebarHeader>
    <SidebarContent>
      <SidebarGroup :aria-label="$t('sidebar.main')">
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem v-for="item in main" :key="item.name">
              <SidebarMenuButton as-child :is-active="current === item.name" :class="itemClass">
                <RouterLink :to="item.to">
                  <span>{{ $t(`nav.${item.name}`) }}</span>
                  <KbdGroup :class="hintClass"><Kbd v-for="key in keys(item.keys)" :key="key">{{ key }}</Kbd></KbdGroup>
                </RouterLink>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
      <SidebarGroup data-testid="sidebar-accounts">
        <SidebarGroupLabel class="text-meta font-normal text-muted-foreground">{{ $t('nav.accounts') }}</SidebarGroupLabel>
        <RouterLink to="/accounts" class="flex h-8 items-center justify-between rounded-md px-2 text-sm hover:bg-sidebar-accent">
          <span class="font-medium">{{ $t('accounts.netWorth') }}</span>
          <MoneyText :amount="accounts.overview.netWorth.net" :currency="false" class="font-semibold" data-testid="sidebar-net-worth" />
        </RouterLink>
        <Collapsible v-for="s in sections" :key="s.group?.id ?? 'none'" default-open class="group/coll">
          <CollapsibleTrigger class="flex h-7 w-full items-center gap-1 rounded-md px-2 text-meta text-muted-foreground hover:bg-sidebar-accent">
            <ChevronDown class="size-3 transition-transform group-data-[state=closed]/coll:-rotate-90" />
            <span class="flex-1 truncate text-left">{{ s.group?.name ?? $t('accounts.noGroup') }}</span>
            <MoneyText :amount="s.subtotal" :currency="false" />
          </CollapsibleTrigger>
          <CollapsibleContent>
            <SidebarMenu>
              <SidebarMenuItem v-for="a in s.accounts" :key="a.id">
                <SidebarMenuButton as-child size="sm" class="pl-6">
                  <RouterLink to="/accounts">
                    <span class="truncate">{{ a.name }}</span>
                    <MoneyText :amount="a.balance" :kind="balanceTone(a)" :currency="false" class="ml-auto" />
                  </RouterLink>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </CollapsibleContent>
        </Collapsible>
      </SidebarGroup>
      <SidebarGroup>
        <SidebarGroupLabel class="text-meta font-normal text-muted-foreground">{{ $t('sidebar.manage') }}</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            <SidebarMenuItem v-for="item in manage" :key="item.name">
              <SidebarMenuButton as-child :is-active="current === item.name" :class="itemClass">
                <RouterLink :to="item.to">
                  <span>{{ $t(`nav.${item.name}`) }}</span>
                </RouterLink>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </SidebarContent>
    <SidebarFooter class="border-t">
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton as-child :is-active="current === settings.name" :class="itemClass">
            <RouterLink :to="settings.to">
              <span>{{ $t(`nav.${settings.name}`) }}</span>
              <KbdGroup :class="hintClass"><Kbd v-for="key in keys(settings.keys)" :key="key">{{ key }}</Kbd></KbdGroup>
            </RouterLink>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarFooter>
  </Sidebar>
</template>
