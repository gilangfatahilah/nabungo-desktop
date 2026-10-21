<script setup lang="ts">
import { EllipsisVertical, Plus } from '@lucide/vue'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import EmptyState from '@/components/EmptyState.vue'
import MoneyText from '@/components/MoneyText.vue'
import PageHeader from '@/components/PageHeader.vue'
import AccountAllocationDialog from '@/components/allocation/AccountAllocationDialog.vue'
import AllocationBar from '@/components/allocation/AllocationBar.vue'
import BucketDialog from '@/components/allocation/BucketDialog.vue'
import DonutChart from '@/components/charts/DonutChart.vue'
import LineChart from '@/components/charts/LineChart.vue'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { AccountDto, BucketDto } from '@/ipc'
import { countsAsLiability } from '@/lib/accounts'
import { formatPeriod } from '@/lib/date'
import { moneyParts } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { formatBp } from '@/lib/percent'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'
import { useAllocationStore } from '@/stores/allocation'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'

const store = useAllocationStore()
const accounts = useAccountsStore()
const settings = useSettingsStore()
const { t } = useI18n()

// Palet chart (ui-guidelines §9.6): root bergiliran --chart-1..4, --chart-5 khusus "Belum dialokasikan".
const PALETTE = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)']
const UNALLOCATED = 'var(--chart-5)'
// Kolom: bucket | target | aktual % | aktual Rp | selisih poin | selisih Rp | bar | menu.
const grid = 'grid grid-cols-[minmax(0,1fr)_4.5rem_4.5rem_8rem_6rem_8rem_8rem_2rem] items-center gap-4 px-2'

const o = computed(() => store.overview)
const byId = computed(() => new Map((o.value?.buckets ?? []).map((b) => [b.id, b])))
const actuals = computed(() => new Map((o.value?.actuals ?? []).map((a) => [a.bucketId, a])))
const roots = computed(() => (o.value?.buckets ?? []).filter((b) => !b.parentId))
const colorOf = (b: BucketDto): string => {
  const root = b.parentId ? byId.value.get(b.parentId)! : b
  return PALETTE[roots.value.indexOf(root) % PALETTE.length]!
}

const pct = (bp: number) => formatBp(bp, settings.locale)
const num = (v: number) => {
  const p = moneyParts(v, settings.locale)
  return p.sign + p.number
}
const signed = (text: string, v: number) => (v > 0 ? `+${text}` : text)
const points = (bp: number) => signed(pct(bp).replace('%', ''), bp)

const warnings = computed(() =>
  (o.value?.warnings ?? []).map((w) =>
    w.parentId
      ? t('allocation.warnChild', { name: byId.value.get(w.parentId)?.name ?? '', total: pct(w.totalBp) })
      : t('allocation.warnRoot', { total: pct(w.totalBp) }),
  ),
)
const segments = computed(() => [
  ...roots.value.map((b) => ({ key: b.id, label: b.name, value: actuals.value.get(b.id)?.actualBp ?? 0, color: colorOf(b) })),
  { key: 'unallocated', label: t('allocation.unallocated'), value: o.value?.unallocatedBp ?? 0, color: UNALLOCATED },
])

// Akun aset aktif, plus akun aset terarsip yang masih bersaldo (masih ikut dihitung di total aset).
const assetAccounts = computed(() =>
  accounts.overview.accounts.filter((a) => !countsAsLiability(a.kind, a.balance) && (!a.archived || a.balance !== 0)),
)
function mappingSummary(accountId: string): string | null {
  const mine = (o.value?.mappings ?? []).filter((m) => m.accountId === accountId)
  if (!mine.length) return null
  return mine.map((m) => `${byId.value.get(m.bucketId)?.name ?? ''} ${pct(m.shareBp)}`).join(' · ')
}

const history = computed(() => store.history)
const historyLabels = computed(() => history.value.map((p) => formatPeriod(p.period, settings.locale)))
const shareAt = (i: number, bucketId: string) =>
  history.value[i]?.buckets.find((b) => b.bucketId === bucketId)?.actualBp ?? 0
const historySeries = computed(() =>
  roots.value.flatMap((b) => [
    { key: b.id, label: b.name, color: colorOf(b), values: history.value.map((_, i) => shareAt(i, b.id)) },
    { key: `${b.id}-target`, label: t('allocation.targetOf', { name: b.name }), color: colorOf(b), values: history.value.map(() => b.targetBp), dashed: true },
  ]),
)

const bucketDialog = ref(false)
const editing = ref<BucketDto | undefined>()
const parent = ref<BucketDto | null>(null)
const deleting = ref<BucketDto | null>(null)
const mapping = ref<AccountDto | null>(null)
const mappingOpen = ref(false)
const deleteOpen = ref(false)

function openBucket(bucket?: BucketDto, under: BucketDto | null = null) {
  editing.value = bucket
  parent.value = bucket?.parentId ? byId.value.get(bucket.parentId)! : under
  bucketDialog.value = true
}
function askDelete(b: BucketDto) {
  deleting.value = b
  deleteOpen.value = true
}
function openMapping(a: AccountDto) {
  mapping.value = a
  mappingOpen.value = true
}

onMounted(() => {
  void withErrorToast(() => Promise.all([store.load(), accounts.load()]))
})

const txns = useTransactionsStore()
watch(() => txns.ledgerVersion, () => void withErrorToast(() => store.load()))
</script>

<template>
  <PageHeader :title="t('nav.allocation')">
    <template v-if="o">
      {{ t('allocation.summary', { gross: num(o.gross), liabilities: num(o.liabilities) }) }}
    </template>
    <template #actions>
      <Button data-testid="bucket-new" @click="openBucket()"><Plus /> {{ t('allocation.newBucket') }}</Button>
    </template>
  </PageHeader>

  <EmptyState v-if="o && !o.buckets.length" :title="t('allocation.empty')" :description="t('allocation.emptyHint')" />
  <div v-else-if="o" class="flex flex-col gap-10">
    <div v-if="warnings.length" data-testid="allocation-warnings" class="-mt-2 flex flex-col gap-1 text-meta text-warning" role="status">
      <p v-for="w in warnings" :key="w">{{ w }}</p>
    </div>

    <section class="flex items-center gap-8">
      <div class="size-36 shrink-0"><DonutChart :segments="segments" :label="t('allocation.donut')" /></div>
      <ul class="flex flex-col gap-1.5 text-meta">
        <li v-for="s in segments.filter((x) => x.value > 0)" :key="s.key" class="flex items-center gap-2">
          <span class="size-2.5 rounded-full" :style="{ background: s.color }" />
          <span>{{ s.label }}</span>
          <span class="num text-muted-foreground">{{ pct(s.value) }}</span>
        </li>
      </ul>
    </section>

    <div class="-mx-2 flex flex-col" role="table">
      <div :class="cn(grid, 'h-8 text-meta whitespace-nowrap text-muted-foreground')" role="row">
        <span role="columnheader">{{ t('allocation.bucket') }}</span>
        <span role="columnheader" class="text-right">{{ t('allocation.target') }}</span>
        <span role="columnheader" class="text-right">{{ t('allocation.actualBp') }}</span>
        <span role="columnheader" class="text-right">{{ t('allocation.actual') }}</span>
        <span role="columnheader" class="text-right">{{ t('allocation.diffBp') }}</span>
        <span role="columnheader" class="text-right">{{ t('allocation.diff') }}</span>
        <span role="columnheader" />
        <span role="columnheader" />
      </div>
      <div
        v-for="b in o.buckets" :key="b.id" role="row" :data-testid="`bucket-row-${b.id}`"
        :class="cn(grid, 'group/row h-12 rounded-lg hover:bg-muted/50')"
      >
        <span :class="cn('min-w-0', b.parentId && 'pl-6')">
          <span :class="cn('block truncate', !b.parentId && 'font-medium')">{{ b.name }}</span>
          <span v-if="b.parentId" class="block truncate text-meta text-muted-foreground">
            {{ t('allocation.relative', { percent: pct(b.targetBp), name: byId.get(b.parentId)?.name ?? '' }) }}
          </span>
        </span>
        <span :data-testid="`bucket-target-${b.id}`" class="num text-right">{{ pct(actuals.get(b.id)?.targetBp ?? 0) }}</span>
        <span class="num text-right">{{ pct(actuals.get(b.id)?.actualBp ?? 0) }}</span>
        <span class="num text-right">{{ num(actuals.get(b.id)?.actual ?? 0) }}</span>
        <span class="num text-right text-muted-foreground">{{ points(actuals.get(b.id)?.diffBp ?? 0) }}</span>
        <span class="num text-right text-muted-foreground">{{ signed(num(actuals.get(b.id)?.diffAmount ?? 0), actuals.get(b.id)?.diffAmount ?? 0) }}</span>
        <AllocationBar :actual-bp="actuals.get(b.id)?.actualBp ?? 0" :target-bp="actuals.get(b.id)?.targetBp ?? 0" :color="colorOf(b)" />
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost" size="icon" :aria-label="t('allocation.menu', { name: b.name })"
              class="size-8 opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100"
            >
              <EllipsisVertical />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem @select="openBucket(b)">{{ t('actions.edit') }}</DropdownMenuItem>
            <DropdownMenuItem v-if="!b.parentId" @select="openBucket(undefined, b)">{{ t('allocation.newChild') }}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" @select="askDelete(b)">{{ t('actions.delete') }}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <div v-if="o.unallocated.length" :class="cn(grid, 'mt-1 h-12 border-t')" role="row" data-testid="bucket-unallocated">
        <span class="flex items-center gap-2 text-muted-foreground">
          <span class="size-2.5 rounded-full" :style="{ background: UNALLOCATED }" />{{ t('allocation.unallocated') }}
        </span>
        <span />
        <span class="num text-right">{{ pct(o.unallocatedBp) }}</span>
        <span class="num text-right">{{ num(o.unallocatedTotal) }}</span>
        <span /><span /><span /><span />
      </div>
    </div>

    <section class="flex flex-col gap-2">
      <h2 class="font-medium">{{ t('allocation.mappings') }}</h2>
      <ul class="-mx-2 flex flex-col">
        <li
          v-for="a in assetAccounts" :key="a.id" :data-testid="`mapping-${a.id}`"
          class="flex h-12 items-center gap-4 rounded-lg px-2 hover:bg-muted/50"
        >
          <span class="w-48 truncate">{{ a.name }}</span>
          <MoneyText :amount="a.balance" class="w-36 text-right" />
          <span v-if="mappingSummary(a.id)" class="flex-1 truncate text-meta text-muted-foreground">{{ mappingSummary(a.id) }}</span>
          <span v-else class="flex-1 text-meta text-warning">{{ t('allocation.unmapped') }}</span>
          <Button variant="outline" size="sm" :data-testid="`map-account-${a.id}`" @click="openMapping(a)">{{ t('allocation.configure') }}</Button>
        </li>
      </ul>
    </section>

    <section v-if="roots.length && history.length" class="flex flex-col gap-4">
      <h2 class="font-medium">{{ t('allocation.history') }}</h2>
      <div class="max-w-3xl"><LineChart :labels="historyLabels" :series="historySeries" :label="t('allocation.historyChart')" :format="pct" /></div>
      <div class="-mx-2 overflow-x-auto" role="table">
        <div class="flex h-8 items-center gap-4 px-2 text-meta text-muted-foreground" role="row">
          <span role="columnheader" class="w-24">{{ t('allocation.period') }}</span>
          <span v-for="b in roots" :key="b.id" role="columnheader" class="w-24 truncate text-right">{{ b.name }}</span>
        </div>
        <div v-for="(p, i) in history" :key="p.period" class="flex h-9 items-center gap-4 px-2" role="row">
          <span class="num w-24">{{ historyLabels[i] }}</span>
          <span v-for="b in roots" :key="b.id" class="num w-24 text-right">{{ pct(shareAt(i, b.id)) }}</span>
        </div>
      </div>
    </section>
  </div>

  <BucketDialog v-model:open="bucketDialog" :bucket="editing" :parent="parent" />
  <AccountAllocationDialog v-if="mapping" v-model:open="mappingOpen" :account="mapping" />
  <ConfirmDialog
    v-model:open="deleteOpen" :title="t('allocation.deleteBucket', { name: deleting?.name ?? '' })"
    :description="t('allocation.deleteHint')" :confirm-label="t('actions.delete')" destructive
    @confirm="deleting && withErrorToast(() => store.deleteBucket(deleting!.id))"
  />
</template>
