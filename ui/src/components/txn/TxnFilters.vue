<script setup lang="ts">
import { Columns3Icon, ListFilterIcon, ListIcon, SearchIcon, Table2Icon, XIcon } from '@lucide/vue'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AccountSelect from '@/components/AccountSelect.vue'
import CategorySelect from '@/components/CategorySelect.vue'
import DateRangeInput from '@/components/DateRangeInput.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuContent, DropdownMenuLabel, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { listTags, parseAmount } from '@/ipc'
import { todayIso } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { formatIdr } from '@/lib/money'
import { activeFilterCount, type Period, periodOf, periodRange } from '@/lib/txnFilter'
import { withErrorToast } from '@/lib/notify'
import { OPTIONAL_COLUMNS, type TxnView } from '@/lib/txnView'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useLabelsStore } from '@/stores/labels'
import { useSettingsStore } from '@/stores/settings'
import { type TxnFilter, useTransactionsStore } from '@/stores/transactions'

// Filter disimpan di store (bukan URL). Payee dicari lewat kotak pencarian; filter payeeId
// (dari layar Tag & Payee) hanya tampil sebagai chip.
// Toolbar satu baris (design.md §7.6): cari, periode, akun selalu terlihat; sisanya di "+ Filter" + chip.
const { t } = useI18n()
const store = useTransactionsStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const labels = useLabelsStore()
const settings = useSettingsStore()

// Reka Select tidak menerima nilai kosong; sentinel untuk "Semua".
const ALL = '__all'

const search = ref(store.query.search ?? '')
const minText = ref(store.query.amountMin?.toString() ?? '')
const maxText = ref(store.query.amountMax?.toString() ?? '')
const amountError = ref<string | null>(null)
const tags = ref<string[]>([])

// "Kustom" dipilih tapi tanggal belum diisi: rentang masih cocok dengan preset lain.
const customOpen = ref(false)
const period = computed<Period>(() => (customOpen.value ? 'custom' : periodOf(store.query, todayIso())))
const PERIODS: Period[] = ['all', 'thisMonth', 'last30', 'custom']
const rangeOpen = ref(false)

function setPeriod(v: unknown) {
  const p = v as Period
  customOpen.value = p === 'custom'
  if (p !== 'custom') apply(periodRange(p, todayIso()))
  // Buka kalender setelah Select selesai menutup dan mengembalikan fokus ke pemicunya.
  else setTimeout(() => { rangeOpen.value = true }, 0)
}

const KINDS = ['expense', 'income', 'transfer'] as const

type Chip = { key: 'categoryId' | 'tag' | 'payeeId' | 'amountMin' | 'amountMax'; label: string }
const chips = computed<Chip[]>(() => {
  const q = store.query
  const out: Chip[] = []
  if (q.categoryId) out.push({ key: 'categoryId', label: categories.list.find((c) => c.id === q.categoryId)?.name ?? '' })
  if (q.tag) out.push({ key: 'tag', label: `#${q.tag}` })
  if (q.payeeId) {
    const name = labels.payees.find((p) => p.id === q.payeeId)?.name ?? ''
    out.push({ key: 'payeeId', label: t('txn.filters.payee', { name }) })
  }
  if (q.amountMin !== null) out.push({ key: 'amountMin', label: `≥ ${formatIdr(q.amountMin, settings.locale)}` })
  if (q.amountMax !== null) out.push({ key: 'amountMax', label: `≤ ${formatIdr(q.amountMax, settings.locale)}` })
  return out
})
const CHIP_IDS: Partial<Record<Chip['key'], string>> = { categoryId: 'category', payeeId: 'payee' }
const hasFilter = computed(() => activeFilterCount(store.query) > 0)

const activeAccounts = computed(() => accounts.overview.accounts.filter((a) => !a.archived || a.id === store.query.accountId))

let timer: ReturnType<typeof setTimeout> | undefined
watch(search, (v) => {
  clearTimeout(timer)
  timer = setTimeout(() => apply({ search: v.trim() || null }), 250)
})
onUnmounted(() => clearTimeout(timer))

// Sinkronkan teks lokal saat query diubah dari luar (mis. Reset).
watch(() => store.query, (q) => {
  if ((q.search ?? '') !== search.value.trim()) search.value = q.search ?? ''
  minText.value = q.amountMin?.toString() ?? ''
  maxText.value = q.amountMax?.toString() ?? ''
})

onMounted(() => withErrorToast(async () => {
  tags.value = await listTags()
  if (store.query.payeeId && !labels.loaded) await labels.reload()
}))

function apply(patch: Partial<TxnFilter>) {
  void withErrorToast(() => store.setQuery(patch))
}

async function applyAmount(field: 'amountMin' | 'amountMax', text: string) {
  if (!text.trim()) {
    amountError.value = null
    apply({ [field]: null })
    return
  }
  try {
    const n = Math.abs(await parseAmount(text))
    amountError.value = null
    apply({ [field]: n })
  } catch (e) {
    amountError.value = errorMessage(e, t as unknown as Translate)
  }
}

const select = (v: unknown) => (v === ALL || v == null ? null : String(v))

function focusPanel(e: Event) {
  e.preventDefault()
  ;(e.target as HTMLElement | null)?.focus()
}

function setView(v: unknown) {
  // ToggleGroup tunggal mengirim kosong saat item aktif diklik lagi: abaikan.
  if (v === 'history' || v === 'table') void withErrorToast(() => store.setView(v as TxnView))
}

function reset() {
  clearTimeout(timer)
  search.value = ''
  amountError.value = null
  customOpen.value = false
  void withErrorToast(() => store.clear())
}
</script>

<template>
  <div class="flex flex-col gap-2 pb-3">
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative w-64">
        <SearchIcon class="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input v-model="search" type="search" class="pl-8" data-testid="filter-search" :placeholder="t('txn.filters.search')" :aria-label="t('txn.filters.search')" />
      </div>
      <Select :model-value="period" @update:model-value="setPeriod">
        <SelectTrigger class="w-44" data-testid="filter-period" :aria-label="t('txn.filters.period')"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem v-for="p in PERIODS" :key="p" :value="p">{{ t(`txn.filters.periods.${p}`) }}</SelectItem>
        </SelectContent>
      </Select>
      <DateRangeInput
        v-if="period === 'custom'" v-model:open="rangeOpen" :from="store.query.dateFrom" :to="store.query.dateTo"
        class="w-64" data-testid="filter-range" :aria-label="t('txn.filters.range')" :placeholder="t('txn.filters.range')"
        @change="apply({ dateFrom: $event.from, dateTo: $event.to })"
      />
      <Select :model-value="store.query.kind ?? ALL" @update:model-value="apply({ kind: select($event) as TxnFilter['kind'] })">
        <SelectTrigger class="w-40" data-testid="filter-kind" :aria-label="t('txn.filters.kind')"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem :value="ALL">{{ t('txn.filters.allKinds') }}</SelectItem>
          <SelectItem v-for="k in KINDS" :key="k" :value="k">{{ t(`txn.kinds.${k}`) }}</SelectItem>
        </SelectContent>
      </Select>
      <AccountSelect
        :model-value="store.query.accountId ?? null" :accounts="activeAccounts" :none-label="t('txn.filters.allAccounts')"
        class="w-40" data-testid="filter-account" :aria-label="t('txn.account')" @update:model-value="apply({ accountId: $event })"
      />
      <Popover>
        <PopoverTrigger as-child>
          <Button variant="outline" data-testid="filter-more"><ListFilterIcon /> {{ t('txn.filters.more') }}</Button>
        </PopoverTrigger>
        <!-- Fokus ke panel, bukan ke combobox kategori yang langsung membuka daftarnya; Tab masuk ke field pertama. -->
        <PopoverContent align="start" class="flex w-72 flex-col gap-2 outline-none" @open-auto-focus="focusPanel">
          <Label for="filter-category">{{ t('txn.category') }}</Label>
          <CategorySelect id="filter-category"
            :model-value="store.query.categoryId" data-testid="filter-category" class="w-full"
            :none-label="t('txn.filters.allCategories')" :aria-label="t('txn.filters.category')"
            @update:model-value="(id: string | null) => { if (id !== store.query.categoryId) apply({ categoryId: id }) }"
          />
          <Label id="filter-tag-label" class="mt-1">{{ t('txn.tags') }}</Label>
          <Select :model-value="store.query.tag ?? ALL" @update:model-value="apply({ tag: select($event) })">
            <SelectTrigger aria-labelledby="filter-tag-label" class="w-full" data-testid="filter-tag"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem :value="ALL">{{ t('txn.filters.allTags') }}</SelectItem>
              <SelectItem v-for="tag in tags" :key="tag" :value="tag">#{{ tag }}</SelectItem>
            </SelectContent>
          </Select>
          <Label class="mt-1">{{ t('txn.amount') }}</Label>
          <div class="grid grid-cols-2 gap-2">
            <Input
              v-model="minText" class="num text-right" inputmode="decimal" data-testid="filter-min" :placeholder="t('txn.filters.min')"
              :aria-label="t('txn.filters.min')" :aria-invalid="!!amountError || undefined" @change="applyAmount('amountMin', minText)"
            />
            <Input
              v-model="maxText" class="num text-right" inputmode="decimal" data-testid="filter-max" :placeholder="t('txn.filters.max')"
              :aria-label="t('txn.filters.max')" :aria-invalid="!!amountError || undefined" @change="applyAmount('amountMax', maxText)"
            />
          </div>
          <p v-if="amountError" class="text-meta text-destructive" role="alert">{{ amountError }}</p>
        </PopoverContent>
      </Popover>
      <p class="num ml-auto text-meta text-muted-foreground" data-testid="filter-count">{{ t('txn.filters.count', { n: store.total }) }}</p>
      <DropdownMenu v-if="store.view === 'table'">
        <DropdownMenuTrigger as-child>
          <Button variant="outline" data-testid="table-columns"><Columns3Icon /> {{ t('txn.columns.menu') }}</Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" class="w-44">
          <DropdownMenuLabel class="text-meta text-muted-foreground">{{ t('txn.columns.menu') }}</DropdownMenuLabel>
          <DropdownMenuCheckboxItem
            v-for="col in OPTIONAL_COLUMNS" :key="col" :data-testid="`column-${col}`"
            :model-value="!store.hiddenColumns.includes(col)"
            @update:model-value="(v: boolean) => store.toggleColumn(col, v)"
            @select.prevent
          >
            {{ t(`txn.columns.${col}`) }}
          </DropdownMenuCheckboxItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <ToggleGroup
        type="single" variant="outline" :model-value="store.view" :aria-label="t('txn.view.label')"
        data-testid="txn-view" @update:model-value="setView"
      >
        <ToggleGroupItem value="history" data-testid="txn-view-history" :aria-label="t('txn.view.history')" :title="t('txn.view.history')">
          <ListIcon />
        </ToggleGroupItem>
        <ToggleGroupItem value="table" data-testid="txn-view-table" :aria-label="t('txn.view.table')" :title="t('txn.view.table')">
          <Table2Icon />
        </ToggleGroupItem>
      </ToggleGroup>
    </div>
    <div v-if="hasFilter" class="flex flex-wrap items-center gap-2">
      <span
        v-for="c in chips" :key="c.key" :data-testid="`chip-${CHIP_IDS[c.key] ?? c.key}`"
        class="inline-flex h-6 items-center gap-1 rounded-full border pr-1 pl-2.5 text-meta"
      >
        {{ c.label }}
        <button
          type="button" class="rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          :aria-label="t('txn.filters.remove', { name: c.label })" @click="apply({ [c.key]: null })"
        >
          <XIcon class="size-3" />
        </button>
      </span>
      <Button variant="ghost" size="sm" class="ml-auto text-muted-foreground" data-testid="filter-reset" @click="reset">{{ t('txn.filters.clearAll') }}</Button>
    </div>
  </div>
</template>
