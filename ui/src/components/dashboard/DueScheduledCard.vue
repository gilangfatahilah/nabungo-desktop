<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import type { RuleDto } from '@/ipc'
import { formatShortDate } from '@/lib/date'
import type { Translate } from '@/lib/errors'
import { ruleName } from '@/lib/recurrence'
import { useCategoriesStore } from '@/stores/categories'
import { useScheduledStore } from '@/stores/scheduled'
import { useSettingsStore } from '@/stores/settings'

// Kejadian terjadwal yang jatuh tempo; aksi catat/lewati tetap di layar Terjadwal (docs/design/design.md §8.1).
const DUE_LIMIT = 5

const scheduled = useScheduledStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const { t } = useI18n()

const catName = (id: string) => categories.list.find((c) => c.id === id)?.name
const name = (r: RuleDto) => ruleName(r, catName, t as unknown as Translate)
const signed = (r: RuleDto) => (r.form.kind === 'expense' ? -r.form.amount : r.form.amount)
const tone = (r: RuleDto) => (r.form.kind === 'transfer' ? 'neutral' : r.form.kind)

const due = computed(() => scheduled.rules
  .filter((r) => !r.ended)
  .flatMap((rule) => rule.due.map((date) => ({ rule, date, name: name(rule) })))
  .sort((a, b) => a.date.localeCompare(b.date) || a.name.localeCompare(b.name)))
const rows = computed(() => due.value.slice(0, DUE_LIMIT))
</script>

<template>
  <Card v-if="due.length" class="gap-4 shadow-xs" data-testid="dashboard-due">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/scheduled" class="hover:underline">{{ t('dashboard.due') }}</RouterLink>
      </CardTitle>
    </CardHeader>
    <CardContent>
      <ul class="flex flex-col">
        <li
          v-for="x in rows" :key="`${x.rule.id}:${x.date}`" data-testid="dashboard-due-row"
          class="flex h-9 items-center gap-3 text-sm"
        >
          <span class="num w-16 shrink-0 text-muted-foreground">{{ formatShortDate(x.date, settings.locale) }}</span>
          <span class="min-w-0 flex-1 truncate" :title="x.name">{{ x.name }}</span>
          <MoneyText :amount="signed(x.rule)" :kind="tone(x.rule)" :currency="false" class="shrink-0 font-medium" />
        </li>
      </ul>
    </CardContent>
    <CardFooter v-if="due.length > DUE_LIMIT">
      <RouterLink to="/scheduled" data-testid="dashboard-due-more" class="text-meta text-muted-foreground hover:underline">
        {{ t('dashboard.dueMore', { n: due.length }) }}
      </RouterLink>
    </CardFooter>
  </Card>
</template>
