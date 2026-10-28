<script setup lang="ts">
import { EllipsisVertical, Loader2Icon, Plus } from '@lucide/vue'
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'
import CaptureRow from '@/components/inbox/CaptureRow.vue'
import CaptureSheet from '@/components/inbox/CaptureSheet.vue'
import DuplicateDialog from '@/components/inbox/DuplicateDialog.vue'
import EmailRuleList from '@/components/inbox/EmailRuleList.vue'
import EmailRuleSheet from '@/components/inbox/EmailRuleSheet.vue'
import IgnoreUntilDialog from '@/components/inbox/IgnoreUntilDialog.vue'
import UnparsedList from '@/components/inbox/UnparsedList.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import * as ipc from '@/ipc'
import type { CaptureDto, EmailRuleDto, EmailRuleInputDto, RuleSavedDto } from '@/ipc'
import { isReady } from '@/lib/capture'
import { withErrorToast } from '@/lib/notify'
import { formatDateHeader } from '@/lib/txnList'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useEmailRulesStore } from '@/stores/emailRules'
import { useInboxStore } from '@/stores/inbox'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'

// Inbox email capture (EML-03, docs/design/design.md §8.10). Tidak ada transaksi tanpa konfirmasi user.
const store = useInboxStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const rules = useEmailRulesStore()
const { t } = useI18n()
const route = useRoute()
const router = useRouter()

type Tab = 'pending' | 'unparsed' | 'rules'
// Tab disimpan di query (`/inbox?tab=rules` dari Pengaturan).
const tab = computed<Tab>({
  get: () => (route.query.tab === 'unparsed' || route.query.tab === 'rules' ? route.query.tab : 'pending'),
  set: (v) => void router.replace({ query: { ...route.query, tab: v === 'pending' ? undefined : v } }),
})
const busy = ref<string | null>(null)
const syncing = ref(false)
const ignoreUntilOpen = ref(false)
const reviewing = ref<CaptureDto | null>(null)
const dupOf = ref<CaptureDto | null>(null)
const ruleSheet = ref<{ rule: EmailRuleDto | null; seed: { input: EmailRuleInputDto; sampleMessageId: string | null } | null } | null>(null)
// Rule yang dikonfirmasi tetap tersimpan setelah dialog menutup (dialog menutup sebelum `confirm`).
const deleting = ref<EmailRuleDto | null>(null)
const deleteOpen = ref(false)

onMounted(() => {
  void withErrorToast(() => store.reload())
  void withErrorToast(() => rules.reload())
  void withErrorToast(() => accounts.ensureLoaded())
  void withErrorToast(() => categories.ensureLoaded())
  void store.listen()
})

// Transaksi dicatat/dihapus di luar Inbox (sheet global `N`, undo) mengubah kandidat duplikat: jangan tawarkan `Catat` basi.
watch(() => useTransactionsStore().ledgerVersion, () => void withErrorToast(() => store.reload()))

const notConfigured = computed(
  () => store.loaded && !store.summary.emailEnabled && !store.pending.length && !store.unparsed.length,
)

/** Kelompok per tanggal; urutan dari backend (terbaru dulu) dipertahankan. */
const groups = computed(() => {
  const out: { date: string; items: CaptureDto[] }[] = []
  for (const c of store.pending) {
    const last = out.at(-1)
    if (last?.date === c.date) last.items.push(c)
    else out.push({ date: c.date, items: [c] })
  }
  return out
})

const ctx = computed(() => ({ accounts: accounts.overview.accounts, categories: categories.list }))

async function run(id: string, action: () => Promise<void>) {
  busy.value = id
  try {
    await withErrorToast(action)
  } finally {
    busy.value = null
  }
}

function undoToast(message: string, ids: string[]) {
  toast(message, {
    action: { label: t('inbox.undo'), onClick: () => void withErrorToast(() => store.unignore(ids)) },
  })
}

const confirm = (c: CaptureDto) =>
  run(c.id, async () => {
    await store.quickConfirm(c)
    toast.success(t('inbox.recorded'))
  })

const ignore = (c: CaptureDto) =>
  run(c.id, async () => {
    await store.ignore(c.id)
    undoToast(t('inbox.ignored'), [c.id])
  })

function ignoreUntil(date: string) {
  void withErrorToast(async () => {
    const ids = await store.ignoreUntil(date)
    if (ids.length) undoToast(t('inbox.ignoredMany', { n: ids.length }), ids)
  })
}

function reparsedToast(saved: RuleSavedDto) {
  if (saved.reparsed.emails > 0) toast.success(t('emailRules.savedReparsed', { n: saved.reparsed.emails }))
}

const toggleRule = (r: EmailRuleDto, on: boolean) => run(r.id, async () => reparsedToast(await rules.setEnabled(r.id, on)))

function duplicateRule(r: EmailRuleDto) {
  const { id: _id, pending: _pending, ...input } = r
  ruleSheet.value = { rule: null, seed: { input: { ...input, name: t('emailRules.copyName', { name: r.name }) }, sampleMessageId: null } }
}

function deleteRule() {
  const r = deleting.value
  if (!r) return
  void run(r.id, async () => {
    await rules.remove(r.id)
    toast(t('emailRules.deleted'), {
      action: { label: t('inbox.undo'), onClick: () => void withErrorToast(async () => reparsedToast(await rules.restore(r.id))) },
    })
  })
}

/** K9: rule baru dari email gagal dibaca, terisi dari saran backend. */
const createFromEmail = (messageId: string) =>
  withErrorToast(async () => {
    ruleSheet.value = { rule: null, seed: { input: await ipc.suggestEmailRule(messageId), sampleMessageId: messageId } }
  })

async function syncNow() {
  syncing.value = true
  try {
    await withErrorToast(async () => {
      const s = await ipc.syncEmail()
      toast.success(t('email.syncDone', { n: s.newItems, m: s.unparsed }))
      await store.reload()
    })
  } finally {
    syncing.value = false
  }
}
</script>

<template>
  <PageHeader>
    <template v-if="!notConfigured" #actions>
      <Button v-if="store.summary.emailEnabled" variant="outline" :disabled="syncing" @click="syncNow">
        <Loader2Icon v-if="syncing" class="size-4 animate-spin" aria-hidden="true" />
        {{ t('email.syncNow') }}
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button variant="ghost" size="icon" :aria-label="t('actions.more')" data-testid="inbox-menu">
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem :disabled="!store.pending.length" @select="ignoreUntilOpen = true">{{ t('inbox.ignoreUntil') }}</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </template>
  </PageHeader>

  <EmptyState v-if="notConfigured" :title="t('inbox.notConfiguredTitle')">
    <Button as-child variant="outline">
      <RouterLink to="/settings">{{ t('inbox.notConfiguredAction') }}</RouterLink>
    </Button>
  </EmptyState>
  <template v-else-if="store.loaded">
    <div class="mb-2 flex items-center justify-between gap-2">
    <Tabs v-model="tab">
      <TabsList>
        <TabsTrigger value="pending" class="gap-2 px-3">
          {{ t('inbox.tabPending') }} <span class="num text-muted-foreground">{{ store.pending.length }}</span>
        </TabsTrigger>
        <TabsTrigger value="unparsed" class="gap-2 px-3">
          {{ t('inbox.tabUnparsed') }} <span class="num text-muted-foreground">{{ store.unparsed.length }}</span>
        </TabsTrigger>
        <TabsTrigger value="rules" class="gap-2 px-3">
          {{ t('emailRules.tab') }} <span class="num text-muted-foreground">{{ rules.rules.length }}</span>
        </TabsTrigger>
      </TabsList>
    </Tabs>
    <Button v-if="tab === 'rules'" data-testid="rule-new" @click="ruleSheet = { rule: null, seed: null }"><Plus /> {{ t('emailRules.new') }}</Button>
    </div>

    <template v-if="tab === 'pending'">
      <EmptyState v-if="!store.pending.length" :title="t('inbox.emptyTitle')" />
      <template v-else>
      <section v-for="g in groups" :key="g.date" :aria-label="formatDateHeader(g.date, settings.locale)">
        <h2 data-testid="inbox-date" :data-date="g.date" class="flex h-8 items-end pb-1.5 text-meta font-medium text-muted-foreground">
          {{ formatDateHeader(g.date, settings.locale) }}
        </h2>
        <ul class="-mx-2 flex flex-col">
          <CaptureRow
            v-for="c in g.items" :key="c.id" :capture="c" :ready="isReady(c, ctx)" :busy="busy === c.id"
            @confirm="confirm(c)" @review="reviewing = c" @ignore="ignore(c)" @duplicates="dupOf = c"
          />
        </ul>
      </section>
      </template>
    </template>
    <UnparsedList v-else-if="tab === 'unparsed'" :items="store.unparsed" @create-rule="createFromEmail" />
    <EmailRuleList
      v-else :rules="rules.rules" :busy="busy"
      @edit="(r) => ruleSheet = { rule: r, seed: null }" @duplicate="duplicateRule" @delete="(r) => { deleting = r; deleteOpen = true }" @toggle="toggleRule"
    />
  </template>

  <IgnoreUntilDialog v-model:open="ignoreUntilOpen" :captures="store.pending" @submit="ignoreUntil" />
  <CaptureSheet :capture="reviewing" @close="reviewing = null" />
  <DuplicateDialog :capture="dupOf" @close="dupOf = null" />
  <EmailRuleSheet :open="ruleSheet !== null" :rule="ruleSheet?.rule ?? null" :seed="ruleSheet?.seed ?? null" @close="ruleSheet = null" />
  <ConfirmDialog
    v-model:open="deleteOpen"
    :title="t('emailRules.delete')"
    :description="deleting ? t('emailRules.deleteConfirm', { name: deleting.name, n: deleting.pending }) : ''"
    :confirm-label="t('emailRules.delete')"
    destructive
    @confirm="deleteRule"
  />
</template>
