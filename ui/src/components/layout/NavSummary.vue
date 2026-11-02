<script setup lang="ts">
import { useRouter } from 'vue-router'
import { Lock } from '@lucide/vue'
import { SidebarMenu, SidebarMenuButton, SidebarMenuItem } from '@/components/ui/sidebar'
import { withErrorToast } from '@/lib/notify'
import { useSecurityStore } from '@/stores/security'

// Footer sidebar: tombol Kunci, hanya bila DB terenkripsi (docs/design/design.md §6.2).
const router = useRouter()
const security = useSecurityStore()

function lockNow() {
  void withErrorToast(async () => {
    await security.lock()
    await security.showLockScreen(router)
  })
}
</script>

<template>
  <SidebarMenu v-if="security.encrypted">
    <SidebarMenuItem>
      <SidebarMenuButton :tooltip="$t('security.lockNow')" data-testid="sidebar-lock" @click="lockNow">
        <Lock />
        <span>{{ $t('security.lockNow') }}</span>
      </SidebarMenuButton>
    </SidebarMenuItem>
  </SidebarMenu>
</template>
