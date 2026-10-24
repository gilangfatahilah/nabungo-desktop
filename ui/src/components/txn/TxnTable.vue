<script setup lang="ts">
import { ArrowDownIcon, ArrowRightIcon, ArrowUpDownIcon, ArrowUpIcon, CircleCheckIcon, PaperclipIcon } from '@lucide/vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { computed, nextTick, onMounted, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import type { TxnRowDto, TxnSortFieldDto } from '@/ipc'
import { useRootFontScale } from '@/composables/useRootFontScale'
import { useTxnRowActions } from '@/composables/useTxnRowActions'
import { useTxnRowDisplay } from '@/composables/useTxnRowDisplay'
import { formatDateInput } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import type { TxnFilter } from '@/lib/txnFilter'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'
import TxnEmpty from './TxnEmpty.vue'
import TxnRowMenu from './TxnRowMenu.vue'

// Tampilan Tabel layar Transaksi (docs/design/design.md §7.13, §8.2): datar di Lembar, gaya sama
// dengan "Transaksi terakhir" di Dasbor. Urutan dari backend (sortir lintas halaman).
// `virtual=false` merender semua baris: khusus test, jsdom tidak mengukur layout.
const props = withDefaults(defineProps<{ virtual?: boolean }>(), { virtual: true })

// Tinggi baris & header dalam px pada ukuran teks 100%; dikali skala karena kelasnya rem.
const ROW = 40
const HEADER = 40

const { t } = useI18n()
const store = useTransactionsStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const { accountName, categoryPath, categoryLabel, tile, title, tone } = useTxnRowDisplay()
const { edit, onKey } = useTxnRowActions()

onMounted(() => withErrorToast(async () => {
  await Promise.all([accounts.loaded ? null : accounts.load(), categories.loaded ? null : categories.load()])
}))

const shown = computed(() => ({
  category: !store.hiddenColumns.includes('category'),
  account: !store.hiddenColumns.includes('account'),
  tags: !store.hiddenColumns.includes('tags'),
}))
// Saldo berjalan hanya bermakna saat satu akun diurutkan per tanggal.
const showRunning = computed(() => store.query.accountId !== null && store.sort.field === 'date')

// --- Sortir ---
const SORTABLE: { field: TxnSortFieldDto; column: 'date' | 'description' | 'amount' }[] = [
  { field: 'date', column: 'date' }, { field: 'payee', column: 'description' }, { field: 'amount', column: 'amount' },
]
const sortOf = (column: string) => SORTABLE.find((s) => s.column === column)!.field

function ariaSort(field: TxnSortFieldDto): 'ascending' | 'descending' | 'none' {
  if (store.sort.field !== field) return 'none'
  return store.sort.desc ? 'descending' : 'ascending'
}

function sortIcon(field: TxnSortFieldDto) {
  if (store.sort.field !== field) return ArrowUpDownIcon
  return store.sort.desc ? ArrowDownIcon : ArrowUpIcon
}

function sortBy(field: TxnSortFieldDto) {
  void withErrorToast(() => store.setSort(field))
}

/** Filter cepat dari isi sel; tidak membuka sheet edit. */
function filterBy(patch: Partial<TxnFilter>) {
  void withErrorToast(() => store.setQuery(patch))
}

// --- Virtualisasi: baris pengisi atas/bawah menjaga struktur <table> ---
const scroller = useTemplateRef<HTMLElement>('scroller')
const scale = useRootFontScale()
const virtualizer = useVirtualizer(computed(() => ({
  count: store.rows.length,
  getScrollElement: () => scroller.value,
  estimateSize: () => ROW * scale.value,
  getItemKey: (i: number) => store.rows[i]?.id ?? i,
  paddingStart: HEADER * scale.value,
  overscan: 12,
})))

const visible = computed(() => props.virtual
  ? virtualizer.value.getVirtualItems().map((v) => ({ row: store.rows[v.index]!, index: v.index }))
  : store.rows.map((row, index) => ({ row, index })))

watch(scale, () => virtualizer.value.measure())

const padding = computed(() => {
  const items = virtualizer.value.getVirtualItems()
  if (!props.virtual || items.length === 0) return { top: 0, bottom: 0 }
  return {
    top: items[0]!.start - HEADER * scale.value,
    bottom: virtualizer.value.getTotalSize() - items.at(-1)!.end,
  }
})

watch(() => virtualizer.value.getVirtualItems().at(-1)?.index, (last) => {
  if (props.virtual && last !== undefined && last >= store.rows.length - 20) void withErrorToast(() => store.loadMore())
})

// Tanggal, Deskripsi, Nominal, ikon, menu + kolom opsional.
const colCount = computed(() => 5 + Number(shown.value.category) + Number(shown.value.account) + Number(shown.value.tags) + Number(showRunning.value))

// --- Keyboard ---
async function moveFocus(from: HTMLElement, delta: number) {
  const idx = store.rows.findIndex((r) => r.id === from.dataset.txnId)
  const target = store.rows[idx + delta]
  if (!target) return
  if (props.virtual) virtualizer.value.scrollToIndex(idx + delta)
  await nextTick()
  scroller.value?.querySelector<HTMLElement>(`[data-txn-id="${target.id}"]`)?.focus()
}

function categoryFilterable(r: TxnRowDto) {
  return r.categoryId !== null && r.kind !== 'transfer'
}
</script>

<template>
  <TxnEmpty v-if="store.rows.length === 0 && !store.loading" />
  <div v-else ref="scroller" class="-mx-2 min-h-0 flex-1 overflow-y-auto px-2" data-testid="txn-table">
    <!-- <table> langsung (bukan komponen Table) agar header lengket mengikuti scroller ini. -->
    <table data-slot="table" class="w-full table-fixed caption-bottom text-sm" :aria-label="t('nav.transactions')">
      <TableHeader class="sticky top-0 z-10 bg-background shadow-[inset_0_-1px_0_var(--border)] [&_tr]:border-b-0">
        <TableRow class="hover:bg-transparent">
          <TableHead
            v-for="col in (['date', 'description'] as const)" :key="col" :aria-sort="ariaSort(sortOf(col))"
            :class="col === 'date' ? 'w-28' : ''" class="text-meta text-muted-foreground"
          >
            <button
              type="button" :data-testid="`sort-${sortOf(col)}`" :aria-label="t('txn.sortBy', { column: t(`txn.columns.${col}`) })"
              class="-ml-1 inline-flex items-center gap-1 rounded-md px-1 py-0.5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              :class="{ 'text-foreground': store.sort.field === sortOf(col) }"
              @click="sortBy(sortOf(col))"
            >
              {{ t(`txn.columns.${col}`) }}
              <component :is="sortIcon(sortOf(col))" class="size-3.5" aria-hidden="true" />
            </button>
          </TableHead>
          <TableHead v-if="shown.category" class="w-[16%] text-meta text-muted-foreground">{{ t('txn.columns.category') }}</TableHead>
          <TableHead v-if="shown.account" class="hidden w-[16%] text-meta text-muted-foreground @3xl/main:table-cell">{{ t('txn.columns.account') }}</TableHead>
          <TableHead v-if="shown.tags" class="hidden w-[12%] text-meta text-muted-foreground @5xl/main:table-cell">{{ t('txn.columns.tags') }}</TableHead>
          <TableHead :aria-sort="ariaSort('amount')" class="w-36 text-right text-meta text-muted-foreground">
            <button
              type="button" data-testid="sort-amount" :aria-label="t('txn.sortBy', { column: t('txn.columns.amount') })"
              class="-mr-1 inline-flex items-center gap-1 rounded-md px-1 py-0.5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              :class="{ 'text-foreground': store.sort.field === 'amount' }"
              @click="sortBy('amount')"
            >
              <component :is="sortIcon('amount')" class="size-3.5" aria-hidden="true" />
              {{ t('txn.columns.amount') }}
            </button>
          </TableHead>
          <TableHead v-if="showRunning" class="w-36 text-right text-meta text-muted-foreground" data-testid="col-balance">{{ t('txn.columns.balance') }}</TableHead>
          <TableHead class="w-12"><span class="sr-only">{{ t('txn.hasAttachment') }}</span></TableHead>
          <TableHead class="w-10"><span class="sr-only">{{ t('actions.more') }}</span></TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <tr v-if="padding.top > 0" aria-hidden="true"><td :colspan="colCount" :style="{ height: `${padding.top}px` }" /></tr>
        <TableRow
          v-for="v in visible"
          :key="v.row.id"
          tabindex="0"
          :data-index="v.index"
          :data-txn-id="v.row.id"
          data-testid="txn-table-row"
          class="group/row h-10 cursor-default outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset"
          @click="edit(v.row)"
          @keydown="onKey($event, v.row, moveFocus)"
        >
          <TableCell class="num text-muted-foreground">{{ formatDateInput(v.row.date, settings.locale) }}</TableCell>
          <TableCell>
            <div class="flex min-w-0 items-center gap-2">
              <span class="flex size-6 shrink-0 items-center justify-center rounded-md" :class="tile(v.row).class">
                <component :is="tile(v.row).icon" class="size-3.5" :aria-label="t(`txn.kinds.${v.row.kind}`)" />
              </span>
              <span class="truncate font-medium" :title="title(v.row)">{{ title(v.row) }}</span>
            </div>
          </TableCell>
          <TableCell v-if="shown.category" class="truncate text-muted-foreground" :title="categoryPath(v.row.categoryId) || undefined" data-testid="cell-category">
            <button
              v-if="categoryFilterable(v.row)" type="button" class="max-w-full truncate rounded-sm hover:text-foreground hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              data-testid="cell-filter-category" :aria-label="t('txn.filterBy', { name: categoryLabel(v.row) })"
              @click.stop="filterBy({ categoryId: v.row.categoryId })"
            >{{ categoryLabel(v.row) }}</button>
            <template v-else>{{ categoryLabel(v.row) }}</template>
          </TableCell>
          <TableCell v-if="shown.account" class="hidden text-muted-foreground @3xl/main:table-cell" data-testid="cell-account">
            <span class="flex min-w-0 items-center gap-1">
              <template v-for="(id, i) in [v.row.accountId, v.row.toAccountId]" :key="i">
                <template v-if="id">
                  <ArrowRightIcon v-if="i === 1" class="size-3 shrink-0" aria-label="→" />
                  <button
                    type="button" class="truncate rounded-sm hover:text-foreground hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
                    data-testid="cell-filter-account" :aria-label="t('txn.filterBy', { name: accountName.get(id) })"
                    @click.stop="filterBy({ accountId: id })"
                  >{{ accountName.get(id) }}</button>
                </template>
              </template>
            </span>
          </TableCell>
          <TableCell v-if="shown.tags" class="hidden text-muted-foreground @5xl/main:table-cell">
            <span class="flex min-w-0 items-center gap-2 overflow-hidden" :title="v.row.tags.map((tag) => `#${tag}`).join(' ') || undefined">
              <button
                v-for="tag in v.row.tags" :key="tag" type="button"
                class="shrink-0 rounded-sm hover:text-foreground hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
                data-testid="cell-filter-tag" :aria-label="t('txn.filterBy', { name: `#${tag}` })"
                @click.stop="filterBy({ tag })"
              >#{{ tag }}</button>
            </span>
          </TableCell>
          <TableCell class="text-right">
            <MoneyText data-testid="txn-amount" class="font-medium" :amount="v.row.amount" :kind="tone(v.row)" />
          </TableCell>
          <TableCell v-if="showRunning" class="text-right text-muted-foreground">
            <MoneyText v-if="v.row.runningBalance !== null" data-testid="txn-running" :amount="v.row.runningBalance" />
          </TableCell>
          <TableCell class="text-muted-foreground">
            <span class="flex items-center justify-end gap-1">
              <PaperclipIcon
                v-if="v.row.attachmentCount > 0" data-testid="txn-attachment" class="size-3.5"
                role="img" :aria-label="t('txn.hasAttachment')"
              ><title>{{ t('txn.hasAttachment') }}</title></PaperclipIcon>
              <CircleCheckIcon
                v-if="v.row.reconciled" data-testid="txn-reconciled" class="size-3.5"
                role="img" :aria-label="t('txn.reconciled')"
              ><title>{{ t('txn.reconciled') }}</title></CircleCheckIcon>
            </span>
          </TableCell>
          <TableCell class="p-1"><TxnRowMenu :row="v.row" /></TableCell>
        </TableRow>
        <tr v-if="padding.bottom > 0" aria-hidden="true"><td :colspan="colCount" :style="{ height: `${padding.bottom}px` }" /></tr>
      </TableBody>
    </table>
  </div>
</template>
