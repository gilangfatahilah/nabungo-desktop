<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { Lock, PiggyBank } from '@lucide/vue'
import { SidebarMenu, SidebarMenuAction, SidebarMenuButton, SidebarMenuItem } from '@/components/ui/sidebar'
import MoneyText from '@/components/MoneyText.vue'
import { formatIdr } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useSecurityStore } from '@/stores/security'
import { useSettingsStore } from '@/stores/settings'

// Footer sidebar: kekayaan bersih (angka tanpa Rp) + kunci bila DB terenkripsi (docs/design/design.md §6.2).
const { t } = useI18n()
const router = useRouter()
const accounts = useAccountsStore()
const security = useSecurityStore()
const settings = useSettingsStore()

const net = computed(() => accounts.overview.netWorth.net)
const tooltip = computed(() => `${t('accounts.netWorth')} ${formatIdr(net.value, settings.locale)}`)

function lockNow() {
  void withErrorToast(async () => {
    await security.lock()
    await security.showLockScreen(router)
  })
}
</script>

<template>
  <SidebarMenu>
    <SidebarMenuItem>
      <SidebarMenuButton as-child size="lg" :tooltip="tooltip">
        <RouterLink to="/accounts">
          <PiggyBank />
          <span class="grid min-w-0 flex-1 leading-tight">
            <span class="truncate text-meta text-muted-foreground">{{ $t('accounts.netWorth') }}</span>
            <MoneyText :amount="net" :currency="false" class="truncate font-semibold" data-testid="sidebar-net-worth" />
          </span>
        </RouterLink>
      </SidebarMenuButton>
      <SidebarMenuAction
        v-if="security.encrypted"
        class="top-3! size-6"
        :aria-label="$t('security.lockNow')"
        :title="$t('security.lockNow')"
        data-testid="sidebar-lock"
        @click="lockNow"
      >
        <Lock />
      </SidebarMenuAction>
    </SidebarMenuItem>
  </SidebarMenu>
</template>
