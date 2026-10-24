<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
import 'vue-sonner/style.css'
import { onBeforeUnmount, onMounted, toRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import AppSidebar from '@/components/layout/AppSidebar.vue'
import SiteHeader from '@/components/layout/SiteHeader.vue'
import CommandPalette from '@/components/CommandPalette.vue'
import IntegrityAlert from '@/components/IntegrityAlert.vue'
import NewTxnSheet from '@/components/NewTxnSheet.vue'
import RuleSheet from '@/components/scheduled/RuleSheet.vue'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { useIdleLock } from '@/composables/useIdleLock'
import { setRecoveryHandler } from '@/ipc'
import { withErrorToast } from '@/lib/notify'
import { shouldIgnoreAction } from '@/lib/overlay'
import { createShortcutHandler, type ShortcutAction } from '@/lib/shortcuts'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useSecurityStore } from '@/stores/security'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

const settings = useSettingsStore()
const ui = useUiStore()
const router = useRouter()
const route = useRoute()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const security = useSecurityStore()

security.installLockHandler(router)
// DB gagal dibuka: cukup pindah layar; tidak ada data store yang perlu dibuang (beda dengan lock).
setRecoveryHandler(() => void router.replace({ name: 'recovery' }))

// Kunci otomatis hanya untuk DB terenkripsi yang sedang terbuka, dan tidak selama recovery key
// yang baru dibuat masih ditampilkan.
useIdleLock(
  toRef(security, 'autoLockMinutes'),
  toRef(security, 'idleLockActive'),
  () =>
    void withErrorToast(async () => {
      await security.lock()
      await security.showLockScreen(router)
    }),
)

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
  else if (a.kind === 'fontScale') void withErrorToast(() => settings.stepFontScale(a.step))
  else ui.paletteOpen = !ui.paletteOpen
}

const onKeydown = createShortcutHandler({ onAction })

onMounted(() => {
  // Terkunci: pengaturan dimuat oleh layar kunci setelah unlock.
  void withErrorToast(async () => {
    await security.refresh()
    if (!security.locked) await settings.load()
  })
  window.addEventListener('keydown', onKeydown)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown))
</script>

<template>
  <template v-if="route.meta.bare">
    <RouterView />
    <Toaster :visible-toasts="1" />
  </template>
  <SidebarProvider v-else :style="{ '--header-height': '3rem' }">
    <AppSidebar />
    <!-- Inset Lembar dengan SiteHeader tetap di atas; scroll hanya di area konten (docs/design/design.md §6.1). -->
    <SidebarInset class="h-svh overflow-hidden md:h-[calc(100svh-1rem)]">
      <SiteHeader />
      <div class="@container/main flex min-h-0 w-full flex-1 flex-col overflow-auto px-4 py-4 md:py-6 lg:px-6">
        <RouterView />
      </div>
    </SidebarInset>
    <CommandPalette />
    <NewTxnSheet />
    <RuleSheet />
    <IntegrityAlert />
    <!-- Saat sheet transaksi (27.5rem = 440px pada ukuran teks 100%) terbuka, toast bergeser ke kiri agar tidak menutup tombol Simpan. -->
    <Toaster :visible-toasts="1" :offset="ui.txnEditor ? { right: '29rem', bottom: '1.5rem' } : undefined" />
  </SidebarProvider>
</template>
