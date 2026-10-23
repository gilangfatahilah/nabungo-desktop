<script setup lang="ts">
import type { Component } from 'vue'
import { useRoute } from 'vue-router'
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import type { RouteName } from '@/router'

export interface NavItem {
  name: RouteName
  to: string
  icon: Component
  /** Hint shortcut sesuai PRD §9, mis. `G D`; handler-nya di App.vue (lib/shortcuts). */
  keys?: string
  /** Badge angka di kanan; tidak dirender bila 0. */
  badge?: number
}

defineProps<{ items: NavItem[]; label?: string }>()

const route = useRoute()
const keys = (k: string) => k.split(/[ +]/)
</script>

<template>
  <SidebarGroup>
    <SidebarGroupLabel v-if="label">{{ label }}</SidebarGroupLabel>
    <SidebarGroupContent>
      <SidebarMenu>
        <SidebarMenuItem v-for="item in items" :key="item.name">
          <SidebarMenuButton as-child :is-active="route.name === item.name" :tooltip="$t(`nav.${item.name}`)">
            <RouterLink :to="item.to">
              <component :is="item.icon" />
              <span>{{ $t(`nav.${item.name}`) }}</span>
              <!-- Petunjuk shortcut hanya saat hover/fokus atau item aktif (docs/design/design.md §6.2). -->
              <KbdGroup
                v-if="item.keys"
                class="ml-auto opacity-0 transition-opacity group-hover/menu-button:opacity-100 group-focus-visible/menu-button:opacity-100 group-data-active/menu-button:opacity-100 group-data-[collapsible=icon]:hidden"
              >
                <Kbd v-for="key in keys(item.keys)" :key="key">{{ key }}</Kbd>
              </KbdGroup>
            </RouterLink>
          </SidebarMenuButton>
          <SidebarMenuBadge v-if="item.badge" class="num" :data-testid="`nav-${item.name}-badge`">{{ item.badge }}</SidebarMenuBadge>
        </SidebarMenuItem>
      </SidebarMenu>
    </SidebarGroupContent>
  </SidebarGroup>
</template>
