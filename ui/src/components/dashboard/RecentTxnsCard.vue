<script setup lang="ts">
import { ArrowRight } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import type { TxnRowDto } from '@/ipc'
import { useTxnRowDisplay } from '@/composables/useTxnRowDisplay'
import { formatShortDate } from '@/lib/date'
import { isEditable } from '@/lib/txnForm'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

// Tabel data (docs/design/design.md §7.13): baris dibuka di sheet edit, sama seperti di layar Transaksi.
defineProps<{ rows: TxnRowDto[] }>()

const { t } = useI18n()
const settings = useSettingsStore()
const ui = useUiStore()
const { accountName, categoryLabel, tile, title, showCategory, tone } = useTxnRowDisplay()

function open(r: TxnRowDto) {
  if (isEditable(r.kind)) ui.openEditTxn(r.id)
}
</script>

<template>
  <Card class="shadow-xs" data-testid="recent-txns">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('dashboard.recent') }}</CardTitle>
      <CardAction>
        <Button variant="ghost" size="sm" as-child>
          <RouterLink to="/transactions">{{ t('dashboard.seeAll') }} <ArrowRight /></RouterLink>
        </Button>
      </CardAction>
    </CardHeader>
    <CardContent>
      <Table>
        <TableHeader>
          <TableRow class="hover:bg-transparent">
            <TableHead class="w-20 text-meta text-muted-foreground">{{ t('dashboard.columns.date') }}</TableHead>
            <TableHead class="text-meta text-muted-foreground">{{ t('dashboard.columns.description') }}</TableHead>
            <TableHead class="hidden text-meta text-muted-foreground @3xl/main:table-cell">{{ t('dashboard.columns.account') }}</TableHead>
            <TableHead class="text-right text-meta text-muted-foreground">{{ t('dashboard.columns.amount') }}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow
            v-for="r in rows"
            :key="r.id"
            tabindex="0"
            data-testid="recent-txn"
            class="h-10 cursor-default outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
            @click="open(r)"
            @keydown.enter="open(r)"
          >
            <TableCell class="num text-muted-foreground">{{ formatShortDate(r.date, settings.locale) }}</TableCell>
            <TableCell class="max-w-0">
              <div class="flex min-w-0 items-center gap-2">
                <span class="flex size-6 shrink-0 items-center justify-center rounded-md" :class="tile(r).class">
                  <component :is="tile(r).icon" class="size-3.5" :aria-label="t(`txn.kinds.${r.kind}`)" />
                </span>
                <span class="truncate font-medium" :title="title(r)">{{ title(r) }}</span>
                <span v-if="showCategory(r)" class="truncate text-meta text-muted-foreground">{{ categoryLabel(r) }}</span>
              </div>
            </TableCell>
            <TableCell class="hidden truncate text-muted-foreground @3xl/main:table-cell">
              {{ accountName.get(r.accountId) }}<template v-if="r.toAccountId"> → {{ accountName.get(r.toAccountId) }}</template>
            </TableCell>
            <TableCell class="text-right"><MoneyText :amount="r.amount" :kind="tone(r)" /></TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </CardContent>
  </Card>
</template>
