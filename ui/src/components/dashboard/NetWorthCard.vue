<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import DonutChart from '@/components/charts/DonutChart.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { AccountKindDto } from '@/ipc'
import { assetsByKind } from '@/lib/accounts'
import { formatCompact, formatIdr } from '@/lib/money'
import { formatBp } from '@/lib/percent'
import { useAccountsStore } from '@/stores/accounts'
import { useSettingsStore } from '@/stores/settings'

// Komposisi kekayaan: porsi aset per jenis akun; total aset/liabilitas/bersih dari backend (docs/design/design.md §8.1).
// Warna tetap per jenis agar tidak bergeser saat satu jenis kosong. Tengah donut = total aset (jumlah segmen) ringkas.
const KIND_COLORS: Partial<Record<AccountKindDto, string>> = {
  cash: 'var(--chart-4)',
  bank: 'var(--chart-1)',
  ewallet: 'var(--chart-2)',
  investment: 'var(--chart-3)',
  receivable: 'var(--chart-5)',
}

const { t } = useI18n()
const accounts = useAccountsStore()
const settings = useSettingsStore()

const nw = computed(() => accounts.overview.netWorth)
// Persen hanya presentasi dari angka backend; dibulatkan ke basis point.
const rows = computed(() => assetsByKind(accounts.overview).map((r) => ({
  ...r,
  bp: nw.value.assets > 0 ? Math.round((r.amount * 10000) / nw.value.assets) : 0,
  color: KIND_COLORS[r.kind] ?? 'var(--chart-5)',
})))
// Segmen yang disorot; hover legenda dan donut berbagi state ini.
const hovered = ref<string | null>(null)
const money = (v: number) => formatIdr(v, settings.locale)
const segments = computed(() => rows.value.map((r) => ({
  key: r.kind, label: t(`accountKinds.${r.kind}`), value: r.amount, color: r.color,
})))
</script>

<template>
  <Card v-if="nw.assets > 0" class="@container/card gap-4 shadow-xs" data-testid="dashboard-networth">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/accounts" class="hover:underline">{{ $t('dashboard.composition') }}</RouterLink>
      </CardTitle>
      <CardDescription class="text-meta">{{ $t('dashboard.compositionHint') }}</CardDescription>
    </CardHeader>
    <CardContent class="flex flex-col gap-4">
      <div class="flex flex-col items-center gap-4 @sm/card:flex-row @sm/card:gap-6">
        <div class="relative size-44 shrink-0 @sm/card:size-28">
          <DonutChart v-model:active="hovered" :segments="segments" :label="$t('dashboard.compositionDonut')" :format="money" :locale="settings.locale" />
          <div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center" data-part="donut-total">
            <span class="text-meta text-muted-foreground">IDR</span>
            <span class="num text-heading font-semibold">{{ formatCompact(nw.assets, settings.locale) }}</span>
          </div>
        </div>
        <ul class="flex w-full min-w-0 flex-1 flex-col gap-1.5 text-sm">
          <li v-for="r in rows" :key="r.kind" class="flex items-center gap-2 transition-opacity" data-testid="networth-row"
            :class="{ 'opacity-50': hovered !== null && hovered !== r.kind }" @mouseenter="hovered = r.kind" @mouseleave="hovered = null"
          >
            <span class="size-2.5 shrink-0 rounded-full" :style="{ background: r.color }" />
            <span class="truncate">{{ $t(`accountKinds.${r.kind}`) }}</span>
            <MoneyText :amount="r.amount" :currency="false" class="ml-auto shrink-0" />
            <span class="num w-12 shrink-0 text-right text-meta text-muted-foreground">{{ formatBp(r.bp, settings.locale) }}</span>
          </li>
        </ul>
      </div>
      <dl class="flex flex-col gap-1 border-t pt-3 text-sm" data-testid="networth-summary">
        <div class="flex justify-between gap-3 text-muted-foreground">
          <dt>{{ $t('dashboard.assets') }}</dt>
          <dd><MoneyText :amount="nw.assets" :currency="false" /></dd>
        </div>
        <div class="flex justify-between gap-3 text-muted-foreground">
          <dt>{{ $t('dashboard.liabilities') }}</dt>
          <dd><MoneyText :amount="nw.liabilities" :currency="false" /></dd>
        </div>
        <div class="flex justify-between gap-3 font-medium">
          <dt>{{ $t('dashboard.netWorth') }}</dt>
          <dd><MoneyText :amount="nw.net" :kind="nw.net < 0 ? 'expense' : 'neutral'" :currency="false" /></dd>
        </div>
      </dl>
    </CardContent>
  </Card>
</template>
