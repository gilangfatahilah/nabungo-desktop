<script setup lang="ts">
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
import { useUiStore } from '@/stores/ui'
import type { RouteName } from '@/router'

const ui = useUiStore()
const router = useRouter()

const nav: { name: RouteName; keys: string }[] = [
  { name: 'dashboard', keys: 'G D' },
  { name: 'transactions', keys: 'G T' },
  { name: 'budget', keys: 'G B' },
  { name: 'allocation', keys: 'G A' },
  { name: 'reports', keys: 'G R' },
  { name: 'accounts', keys: '' },
  { name: 'categories', keys: '' },
  { name: 'scheduled', keys: '' },
  { name: 'settings', keys: 'Ctrl+,' },
]

function go(name: RouteName) {
  ui.paletteOpen = false
  void router.push({ name })
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
      </CommandGroup>
    </CommandList>
  </CommandDialog>
</template>
