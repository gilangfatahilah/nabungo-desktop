<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { Search } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { Separator } from '@/components/ui/separator'
import { SidebarTrigger } from '@/components/ui/sidebar'
import { useUiStore } from '@/stores/ui'

// Header aplikasi: toggle sidebar, judul halaman (satu-satunya h1), aksi global (docs/design/design.md §6.3).
const route = useRoute()
const ui = useUiStore()
const titleKey = computed(() => (typeof route.name === 'string' ? `nav.${route.name}` : null))
</script>

<template>
  <header class="flex h-(--header-height) shrink-0 items-center border-b">
    <div class="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
      <SidebarTrigger class="-ml-1" />
      <Separator orientation="vertical" class="mx-2 data-[orientation=vertical]:h-4" />
      <h1 class="shrink-0 text-heading font-medium">{{ titleKey ? $t(titleKey) : '' }}</h1>
      <div class="ml-auto flex min-w-0 items-center gap-2">
        <!-- Tampil seperti kolom isian agar mudah ditemukan; membuka command palette. -->
        <Button
          variant="outline"
          data-testid="open-palette"
          class="h-9 w-56 min-w-0 shrink justify-start bg-muted/40 px-3 font-normal text-muted-foreground shadow-none lg:w-80"
          @click="ui.paletteOpen = true"
        >
          <Search />
          <span class="flex-1 truncate text-left">{{ $t('sidebar.search') }}…</span>
          <KbdGroup><Kbd>Ctrl</Kbd><Kbd>K</Kbd></KbdGroup>
        </Button>
      </div>
    </div>
  </header>
</template>
