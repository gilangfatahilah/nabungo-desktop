<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
import 'vue-sonner/style.css'
import { onBeforeUnmount, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AppSidebar from '@/components/AppSidebar.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import NewTxnSheet from '@/components/NewTxnSheet.vue'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { withErrorToast } from '@/lib/notify'
import { shouldIgnoreAction } from '@/lib/overlay'
import { createShortcutHandler, type ShortcutAction } from '@/lib/shortcuts'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()
const router = useRouter()
const route = useRoute()
const accounts = useAccountsStore()
const categories = useCategoriesStore()

// Akun & kategori baru dimuat setelah onboarding selesai (layar `bare` tidak menampilkan sidebar).
watch(
  () => route.name !== undefined && !route.meta.bare,
  (ready) => {
    if (ready && !accounts.loaded) void withErrorToast(() => accounts.load())
    if (ready && !categories.loaded) void withErrorToast(() => categories.load())
  },
  { immediate: true },
)

function onAction(a: ShortcutAction) {
  // Layar `bare` (onboarding) tidak punya navigasi.
  if (route.meta.bare) return
  if (shouldIgnoreAction(a, ui.newTxnOpen)) return
  if (a.kind === 'navigate') {
    ui.paletteOpen = false
    void router.push({ name: a.route })
  } else if (a.kind === 'newTxn') ui.openNewTxn()
  else ui.paletteOpen = !ui.paletteOpen
}

const onKeydown = createShortcutHandler({ onAction })

onMounted(() => {
  void withErrorToast(() => settings.load())
  window.addEventListener('keydown', onKeydown)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <template v-if="route.meta.bare">
    <RouterView />
    <Toaster :visible-toasts="1" />
  </template>
  <SidebarProvider v-else>
    <AppSidebar />
    <!-- Konten rata kiri, max-width 1120 + padding 32 (docs/design/ui-guidelines.md §7.1). -->
    <SidebarInset class="h-screen overflow-auto bg-background">
      <div
        class="flex min-h-0 w-full flex-1 flex-col px-8 pt-6 pb-8"
        :class="!route.meta.wide && 'max-w-[calc(1120px+4rem)]'"
      >
        <RouterView />
      </div>
    </SidebarInset>
    <CommandPalette />
    <NewTxnSheet />
    <!-- Saat sheet transaksi (440px) terbuka, toast bergeser ke kiri agar tidak menutup tombol Simpan. -->
    <Toaster :visible-toasts="1" :offset="ui.txnEditor ? { right: 464, bottom: 24 } : undefined" />
  </SidebarProvider>
</template>
