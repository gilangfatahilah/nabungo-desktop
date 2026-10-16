<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
import 'vue-sonner/style.css'
import { onBeforeUnmount, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import AppSidebar from '@/components/AppSidebar.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import NewTxnSheet from '@/components/NewTxnSheet.vue'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { createShortcutHandler, type ShortcutAction } from '@/lib/shortcuts'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()
const router = useRouter()

function onAction(a: ShortcutAction) {
  if (a.kind === 'navigate') void router.push({ name: a.route })
  else if (a.kind === 'newTxn') ui.newTxnOpen = true
  else ui.paletteOpen = !ui.paletteOpen
}

const onKeydown = createShortcutHandler({ onAction })

onMounted(() => {
  void settings.load()
  window.addEventListener('keydown', onKeydown)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <SidebarProvider>
    <AppSidebar />
    <SidebarInset class="h-screen overflow-auto">
      <main class="flex-1 p-6">
        <RouterView />
      </main>
    </SidebarInset>
    <CommandPalette />
    <NewTxnSheet />
    <Toaster />
  </SidebarProvider>
</template>
