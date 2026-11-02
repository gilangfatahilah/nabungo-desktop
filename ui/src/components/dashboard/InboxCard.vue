<script setup lang="ts">
import { Mail } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import type { CaptureDto } from '@/ipc'
import { formatShortDate } from '@/lib/date'
import { useInboxStore } from '@/stores/inbox'
import { useSettingsStore } from '@/stores/settings'

// Pintasan Inbox email capture: hitungan + beberapa draft teratas; aksi catat/tinjau tetap di layar Inbox
// (docs/design/design.md §8.1). Tampil dengan aturan yang sama seperti menu sidebar Inbox.
const PENDING_LIMIT = 4

const inbox = useInboxStore()
const settings = useSettingsStore()
const { t } = useI18n()

const rows = computed(() => inbox.pending.slice(0, PENDING_LIMIT))
const signed = (c: CaptureDto) => (c.kind === 'expense' ? -c.amount : c.amount)
const tone = (c: CaptureDto) => (c.kind === 'transfer' ? 'neutral' : c.kind)
</script>

<template>
  <Card v-if="inbox.showNav" class="gap-4 shadow-xs" data-testid="dashboard-inbox">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/inbox" class="hover:underline">{{ t('nav.inbox') }}</RouterLink>
      </CardTitle>
      <CardDescription>
        <template v-if="inbox.summary.pending">{{ t('dashboard.inboxPending', { n: inbox.summary.pending }) }}</template>
        <template v-else>{{ t('inbox.emptyTitle') }}</template>
        <RouterLink
          v-if="inbox.summary.unparsed" to="/inbox?tab=unparsed" data-testid="dashboard-inbox-unparsed"
          class="text-warning hover:underline"
        >
          · {{ t('dashboard.inboxUnparsed', { n: inbox.summary.unparsed }) }}
        </RouterLink>
      </CardDescription>
      <CardAction>
        <Button as-child size="sm" variant="outline" data-testid="dashboard-inbox-open">
          <RouterLink to="/inbox"><Mail /> {{ t('dashboard.inboxOpen') }}</RouterLink>
        </Button>
      </CardAction>
    </CardHeader>
    <CardContent v-if="rows.length">
      <ul class="flex flex-col">
        <li v-for="c in rows" :key="c.id" data-testid="dashboard-inbox-row" class="flex h-9 items-center gap-3 text-sm">
          <span class="num w-16 shrink-0 text-muted-foreground">{{ formatShortDate(c.date, settings.locale) }}</span>
          <span class="min-w-0 flex-1 truncate" :title="c.payee ?? c.ruleName ?? ''">{{ c.payee ?? c.ruleName }}</span>
          <MoneyText :amount="signed(c)" :kind="tone(c)" :currency="false" class="shrink-0 font-medium" />
        </li>
      </ul>
    </CardContent>
    <CardFooter v-if="inbox.pending.length > PENDING_LIMIT">
      <RouterLink to="/inbox" data-testid="dashboard-inbox-more" class="text-meta text-muted-foreground hover:underline">
        {{ t('dashboard.dueMore', { n: inbox.pending.length }) }}
      </RouterLink>
    </CardFooter>
  </Card>
</template>
