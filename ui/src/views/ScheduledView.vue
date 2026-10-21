<script setup lang="ts">
import { EllipsisVertical, Plus } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import EmptyState from '@/components/EmptyState.vue'
import MoneyText from '@/components/MoneyText.vue'
import PageHeader from '@/components/PageHeader.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import type { RuleDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { describeRecurrence, ruleName } from '@/lib/recurrence'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useScheduledStore } from '@/stores/scheduled'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

// Transaksi terjadwal (TRX-06, K13). Kejadian ditangani berurutan: hanya yang tertua per jadwal aktif.
const store = useScheduledStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const ui = useUiStore()
const { t } = useI18n()
const reveal = 'opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100'

onMounted(() => {
  // Selalu muat ulang: tanggal berganti dan auto-post berjalan saat daftar dimuat.
  void withErrorToast(() => store.reload())
  if (!accounts.loaded) void withErrorToast(() => accounts.load())
  if (!categories.loaded) void withErrorToast(() => categories.load())
})

const catName = (id: string) => categories.list.find((c) => c.id === id)?.name
const name = (r: RuleDto) => ruleName(r, catName, t)
const accountName = (id: string) => accounts.overview.accounts.find((a) => a.id === id)?.name ?? ''
const date = (d: string) => formatDateInput(d, settings.locale)
const signed = (r: RuleDto) => (r.form.kind === 'expense' ? -r.form.amount : r.form.amount)
const tone = (r: RuleDto) => (r.form.kind === 'transfer' ? 'neutral' : r.form.kind)

interface DueItem { rule: RuleDto; date: string; first: boolean }
const due = computed<DueItem[]>(() => store.rules
  .flatMap((rule) => rule.due.map((d, i) => ({ rule, date: d, first: i === 0 })))
  .sort((a, b) => a.date.localeCompare(b.date) || a.rule.id.localeCompare(b.rule.id)))

async function record(x: DueItem) {
  await withErrorToast(async () => {
    await store.confirm(x.rule.id, x.date)
    toast.success(t('scheduled.recorded'))
  })
}

async function skip(x: DueItem) {
  await withErrorToast(async () => {
    await store.skip(x.rule.id, x.date)
    toast.success(t('scheduled.skipped'))
  })
}

async function confirmAll(x: DueItem) {
  await withErrorToast(async () => {
    const n = await store.confirmAll(x.rule.id)
    toast.success(t('scheduled.recordedAll', { n }))
  })
}

// --- hapus (transaksi yang sudah tercatat tetap ada) ---
const deleting = ref<RuleDto | null>(null)
const confirmOpen = ref(false)

function remove(r: RuleDto) {
  deleting.value = r
  confirmOpen.value = true
}

function onConfirm() {
  const r = deleting.value
  if (r) void withErrorToast(() => store.remove(r.id))
}
</script>

<template>
  <PageHeader :title="t('nav.scheduled')">
    <template #actions>
      <Button data-testid="rule-new" @click="ui.openRuleEditor({ mode: 'create' })"><Plus /> {{ t('scheduled.new') }}</Button>
    </template>
  </PageHeader>
  <EmptyState v-if="store.loaded && !store.rules.length" :title="t('scheduled.empty')" :description="t('scheduled.emptyHint')" />
  <template v-else>
    <section v-if="due.length" class="mb-8">
      <h2 class="mb-2 text-meta font-medium text-muted-foreground">{{ t('scheduled.dueTitle') }}</h2>
      <TooltipProvider :delay-duration="300">
        <ul class="-mx-2 flex flex-col">
          <li
            v-for="x in due" :key="`${x.rule.id}:${x.date}`" data-testid="due-row"
            class="group/row flex h-12 items-center gap-3 rounded-lg px-2 hover:bg-muted"
          >
            <span class="num w-28 shrink-0 text-muted-foreground">{{ date(x.date) }}</span>
            <span class="min-w-0 flex-1 truncate font-medium">{{ name(x.rule) }}</span>
            <span class="hidden shrink-0 truncate text-meta text-muted-foreground sm:inline">{{ accountName(x.rule.form.accountId) }}</span>
            <MoneyText :amount="signed(x.rule)" :kind="tone(x.rule)" class="w-32 shrink-0 text-right font-medium" />
            <Tooltip :disabled="x.first">
              <TooltipTrigger as-child>
                <span class="flex shrink-0 items-center gap-1" :tabindex="x.first ? undefined : 0">
                  <Button size="sm" data-testid="due-confirm" :disabled="!x.first" @click="record(x)">{{ t('scheduled.record') }}</Button>
                  <Button size="sm" variant="ghost" data-testid="due-skip" :disabled="!x.first" @click="skip(x)">{{ t('scheduled.skip') }}</Button>
                </span>
              </TooltipTrigger>
              <TooltipContent>{{ t('scheduled.handlePrevious') }}</TooltipContent>
            </Tooltip>
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <Button
                  variant="ghost" size="icon" :class="cn('size-8', reveal)" :aria-label="t('actions.more')"
                  data-testid="due-menu" :disabled="!x.first"
                >
                  <EllipsisVertical />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem data-testid="due-edit" @select="ui.openRuleEditor({ mode: 'occurrence', ruleId: x.rule.id, date: x.date })">
                  {{ t('scheduled.editThenRecord') }}
                </DropdownMenuItem>
                <DropdownMenuItem v-if="x.rule.due.length > 1" data-testid="due-confirm-all" @select="confirmAll(x)">
                  {{ t('scheduled.confirmAll', { n: x.rule.due.length }) }}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </li>
        </ul>
      </TooltipProvider>
    </section>
    <section>
      <h2 class="mb-2 text-meta font-medium text-muted-foreground">{{ t('scheduled.rulesTitle') }}</h2>
      <ul class="-mx-2 flex flex-col">
        <li
          v-for="r in store.rules" :key="r.id" data-testid="rule-row"
          class="group/row flex h-12 items-center gap-3 rounded-lg px-2 hover:bg-muted"
        >
          <span class="flex min-w-0 flex-1 items-baseline gap-2">
            <span class="truncate font-medium" :class="{ 'text-muted-foreground': r.ended }">{{ name(r) }}</span>
            <span class="truncate text-meta text-muted-foreground">{{ describeRecurrence(r.recurrence, r.form.date, t, settings.locale) }}</span>
          </span>
          <Badge v-if="r.autoPost && !r.ended" variant="secondary">{{ t('scheduled.auto') }}</Badge>
          <Badge v-if="r.ended" variant="outline">{{ t('scheduled.ended') }}</Badge>
          <span v-if="!r.ended" class="num hidden shrink-0 text-meta text-muted-foreground sm:inline">
            {{ t('scheduled.nextOn', { date: date(r.form.date) }) }}
          </span>
          <MoneyText :amount="signed(r)" :kind="tone(r)" class="w-32 shrink-0 text-right font-medium" />
          <DropdownMenu>
            <DropdownMenuTrigger as-child>
              <Button variant="ghost" size="icon" :class="cn('size-8', reveal)" :aria-label="t('actions.more')" :data-testid="`rule-menu-${r.id}`">
                <EllipsisVertical />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem @select="ui.openRuleEditor({ mode: 'edit', id: r.id })">{{ t('actions.edit') }}</DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" @select="remove(r)">{{ t('actions.delete') }}</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </li>
      </ul>
    </section>
  </template>

  <ConfirmDialog
    v-if="deleting"
    v-model:open="confirmOpen"
    :title="t('scheduled.deleteTitle', { name: name(deleting) })"
    :description="t('scheduled.deleteBody')"
    :confirm-label="t('actions.delete')"
    destructive
    @confirm="onConfirm"
  />
</template>
