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
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()
const router = useRouter()
const route = useRoute()
const accounts = useAccountsStore()

// Akun baru dimuat setelah onboarding selesai (layar `bare` tidak menampilkan sidebar).
watch(
  () => route.name !== undefined && !route.meta.bare,
  (ready) => {
    if (ready && !accounts.loaded) void withErrorToast(() => accounts.load())
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
  } else if (a.kind === 'newTxn') ui.newTxnOpen = true
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
    <Toaster />
  </template>
  <SidebarProvider v-else>
    <AppSidebar />
    <SidebarInset class="h-screen overflow-auto">
      <div class="flex-1 p-6">
        <RouterView />
      </div>
    </SidebarInset>
    <CommandPalette />
    <NewTxnSheet />
    <Toaster />
  </SidebarProvider>
</template>
