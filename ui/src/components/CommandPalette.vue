<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from '@/components/ui/command'
import { useInboxStore } from '@/stores/inbox'
import { useUiStore } from '@/stores/ui'
import type { RouteName } from '@/router'

const ui = useUiStore()
const router = useRouter()

const inbox = useInboxStore()
const nav = computed<{ name: RouteName; keys: string }[]>(() => [
  { name: 'dashboard', keys: 'G D' },
  { name: 'transactions', keys: 'G T' },
  { name: 'budget', keys: 'G B' },
  { name: 'allocation', keys: 'G A' },
  { name: 'reports', keys: 'G R' },
  { name: 'accounts', keys: '' },
  { name: 'categories', keys: '' },
  { name: 'scheduled', keys: '' },
  ...(inbox.showNav ? [{ name: 'inbox' as const, keys: '' }] : []),
  { name: 'settings', keys: 'Ctrl+,' },
])

function go(name: RouteName) {
  ui.paletteOpen = false
  void router.push({ name })
}

function toggleAmounts() {
  ui.paletteOpen = false
  ui.toggleAmountsHidden()
}

function newTxn() {
  ui.paletteOpen = false
  ui.openNewTxn()
}
</script>

<template>
  <CommandDialog
    v-model:open="ui.paletteOpen"
    :title="$t('palette.title')"
    :description="$t('palette.description')"
  >
    <CommandInput :placeholder="$t('palette.placeholder')" />
    <CommandList>
      <CommandEmpty>{{ $t('palette.empty') }}</CommandEmpty>
      <CommandGroup :heading="$t('palette.navigation')">
        <CommandItem v-for="item in nav" :key="item.name" :value="$t(`nav.${item.name}`)" @select="go(item.name)">
          <span>{{ $t(`nav.${item.name}`) }}</span>
          <CommandShortcut v-if="item.keys">{{ item.keys }}</CommandShortcut>
        </CommandItem>
      </CommandGroup>
      <CommandGroup :heading="$t('palette.actions')">
        <CommandItem :value="$t('txn.new')" @select="newTxn">
          <span>{{ $t('txn.new') }}</span>
          <CommandShortcut>N</CommandShortcut>
        </CommandItem>
        <CommandItem :value="$t(ui.amountsHidden ? 'privacy.show' : 'privacy.hide')" @select="toggleAmounts">
          <span>{{ $t(ui.amountsHidden ? 'privacy.show' : 'privacy.hide') }}</span>
          <CommandShortcut>Ctrl+Shift+H</CommandShortcut>
        </CommandItem>
      </CommandGroup>
    </CommandList>
  </CommandDialog>
</template>
