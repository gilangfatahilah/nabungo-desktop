<script setup lang="ts">
import PageHeader from '@/components/PageHeader.vue'
import { ChevronRight, EllipsisVertical, Plus } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AccountFormDialog from '@/components/AccountFormDialog.vue'
import AccountList from '@/components/AccountList.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import MoneyText from '@/components/MoneyText.vue'
import ReconcileDialog from '@/components/ReconcileDialog.vue'
import ValuationDialog from '@/components/ValuationDialog.vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { IpcFailure, type AccountDto, type AccountGroupDto } from '@/ipc'
import { groupAccounts, moveAccount } from '@/lib/accounts'
import { errorMessage, type Translate } from '@/lib/errors'
import { formatIdr } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useSettingsStore } from '@/stores/settings'

const store = useAccountsStore()
const settings = useSettingsStore()
const { t } = useI18n()

const sections = computed(() => groupAccounts(store.overview, { archived: false }))
const archived = computed(() => groupAccounts(store.overview, { archived: true })[0]?.accounts ?? [])
const groups = computed(() => sections.value.flatMap((s) => (s.group ? [s.group] : [])))

onMounted(() => {
  if (!store.loaded) void withErrorToast(() => store.load())
})

// --- form akun ---
const formOpen = ref(false)
const editing = ref<AccountDto | undefined>()
const formGroup = ref<string | null>(null)

function newAccount(groupId: string | null = null) {
  editing.value = undefined
  formGroup.value = groupId
  formOpen.value = true
}

function edit(a: AccountDto) {
  editing.value = a
  formOpen.value = true
}

// --- urutan ---
const valuationOpen = ref(false)
const valuating = ref<AccountDto | null>(null)

function valuate(a: AccountDto) {
  valuating.value = a
  valuationOpen.value = true
}

const reconcileOpen = ref(false)
const reconciling = ref<AccountDto | null>(null)
function reconcile(a: AccountDto) {
  reconciling.value = a
  reconcileOpen.value = true
}

function move(accountId: string, groupId: string | null, index: number) {
  void withErrorToast(() => store.reorderAccounts(groupId, moveAccount(sections.value, accountId, groupId, index)))
}

function moveGroup(g: AccountGroupDto, delta: number) {
  const ids = groups.value.map((x) => x.id)
  const i = ids.indexOf(g.id)
  ids.splice(i, 1)
  ids.splice(i + delta, 0, g.id)
  void withErrorToast(() => store.reorderAccountGroups(ids))
}

// --- konfirmasi: satu dialog untuk hapus / arsip / hapus grup ---
interface Pending {
  title: string
  description: string
  confirmLabel: string
  destructive: boolean
  run: () => Promise<unknown>
}
const pending = ref<Pending | null>(null)
const confirmOpen = ref(false)

function ask(p: Pending) {
  pending.value = p
  confirmOpen.value = true
}

function onConfirm() {
  const p = pending.value
  if (p) void withErrorToast(p.run)
}

function offerArchive(a: AccountDto) {
  ask({
    title: t('accounts.inUseTitle'),
    description: t('accounts.inUseBody'),
    confirmLabel: t('actions.archive'),
    destructive: false,
    run: () => store.archiveAccount(a.id),
  })
}

function remove(a: AccountDto) {
  ask({
    title: t('accounts.deleteTitle'),
    description: t('accounts.deleteBody', { name: a.name }),
    confirmLabel: t('actions.delete'),
    destructive: true,
    run: async () => {
      try {
        await store.deleteAccount(a.id)
      } catch (e) {
        // Akun bertransaksi tidak bisa dihapus (acceptance "hapus aman"): tawarkan arsip.
        if (e instanceof IpcFailure && e.error.code === 'in_use') offerArchive(a)
        else throw e
      }
    },
  })
}

function archive(a: AccountDto) {
  if (a.balance === 0) {
    void withErrorToast(() => store.archiveAccount(a.id))
    return
  }
  ask({
    title: t('accounts.archiveTitle'),
    description: t('accounts.archiveBalanceBody', { amount: formatIdr(a.balance, settings.locale) }),
    confirmLabel: t('actions.archive'),
    destructive: false,
    run: () => store.archiveAccount(a.id),
  })
}

function restore(a: AccountDto) {
  void withErrorToast(() => store.unarchiveAccount(a.id))
}

function removeGroup(g: AccountGroupDto) {
  ask({
    title: t('accounts.deleteGroup'),
    description: t('accounts.deleteGroupBody'),
    confirmLabel: t('actions.delete'),
    destructive: true,
    run: () => store.deleteAccountGroup(g.id),
  })
}

// --- dialog nama grup (buat / ubah nama) ---
const groupOpen = ref(false)
const groupTarget = ref<AccountGroupDto | null>(null)
const groupName = ref('')
const groupError = ref<string | null>(null)

function openGroup(g: AccountGroupDto | null) {
  groupTarget.value = g
  groupName.value = g?.name ?? ''
  groupError.value = null
  groupOpen.value = true
}

async function saveGroup() {
  if (groupName.value.trim() === '') {
    groupError.value = t('errors.name_empty')
    return
  }
  try {
    if (groupTarget.value) await store.renameAccountGroup(groupTarget.value.id, groupName.value)
    else await store.createAccountGroup(groupName.value)
    groupOpen.value = false
  } catch (e) {
    groupError.value = errorMessage(e, t as unknown as Translate)
  }
}
</script>

<template>
  <PageHeader :title="t('nav.accounts')">
    {{ t('accounts.netWorth') }} <MoneyText :amount="store.overview.netWorth.net" class="font-medium text-foreground" />
    <template #actions>
      <Button variant="outline" @click="openGroup(null)">{{ t('accounts.newGroup') }}</Button>
      <Button @click="newAccount()"><Plus /> {{ t('accounts.new') }}</Button>
    </template>
  </PageHeader>
  <!-- Tabel datar per grup, header grup dengan subtotal (docs/design/design.md §8.3). -->
  <div class="-mx-2 flex flex-col gap-6">
    <section v-for="(s, si) in sections" :key="s.group?.id ?? 'none'">
      <div data-testid="account-group" class="group/head flex h-10 items-center gap-3 border-b px-2">
        <h2 class="flex-1 truncate font-medium">{{ s.group?.name ?? t('accounts.noGroup') }}</h2>
        <MoneyText data-testid="group-subtotal" :amount="s.subtotal" class="font-medium" />
        <DropdownMenu>
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost" size="icon" class="size-8 opacity-0 group-hover/head:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
              :aria-label="t('actions.more')"
            >
              <EllipsisVertical />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem @select="newAccount(s.group?.id ?? null)">{{ t('accounts.new') }}</DropdownMenuItem>
            <template v-if="s.group">
              <DropdownMenuItem @select="openGroup(s.group)">{{ t('actions.rename') }}</DropdownMenuItem>
              <DropdownMenuItem :disabled="si === 0" @select="moveGroup(s.group, -1)">{{ t('actions.moveUp') }}</DropdownMenuItem>
              <DropdownMenuItem :disabled="si === groups.length - 1" @select="moveGroup(s.group, 1)">
                {{ t('actions.moveDown') }}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" @select="removeGroup(s.group)">{{ t('accounts.deleteGroup') }}</DropdownMenuItem>
            </template>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <AccountList
        :group-id="s.group?.id ?? null"
        :accounts="s.accounts"
        @move="move"
        @edit="edit"
        @valuate="valuate"
        @reconcile="reconcile"
        @archive="archive"
        @delete="remove"
      />
    </section>

    <Collapsible v-if="archived.length" v-slot="{ open }">
      <CollapsibleTrigger as-child>
        <Button variant="ghost" size="sm" data-testid="archived-toggle" class="text-muted-foreground">
          <ChevronRight class="transition-transform" :class="open && 'rotate-90'" />
          {{ t('accounts.archived') }} ({{ archived.length }})
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <AccountList :group-id="null" :accounts="archived" archived @restore="restore" @delete="remove" />
      </CollapsibleContent>
    </Collapsible>

    <AccountFormDialog v-model:open="formOpen" :account="editing" :group-id="formGroup" />
    <ValuationDialog v-if="valuating" v-model:open="valuationOpen" :account="valuating" />
    <ReconcileDialog v-if="reconciling" v-model:open="reconcileOpen" :account="reconciling" />

    <ConfirmDialog
      v-if="pending"
      v-model:open="confirmOpen"
      :title="pending.title"
      :description="pending.description"
      :confirm-label="pending.confirmLabel"
      :destructive="pending.destructive"
      @confirm="onConfirm"
    />

    <Dialog v-model:open="groupOpen">
      <DialogContent class="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{{ groupTarget ? t('accounts.renameGroup') : t('accounts.newGroup') }}</DialogTitle>
          <DialogDescription class="sr-only">{{ t('accounts.groupName') }}</DialogDescription>
        </DialogHeader>
        <form class="flex flex-col gap-3" @submit.prevent="saveGroup">
          <Input v-model="groupName" :aria-label="t('accounts.groupName')" maxlength="80" autofocus />
          <p v-if="groupError" class="text-sm text-destructive" role="alert">{{ groupError }}</p>
          <DialogFooter>
            <Button type="button" variant="ghost" @click="groupOpen = false">{{ t('actions.cancel') }}</Button>
            <Button type="submit">{{ t('actions.save') }}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </div>
</template>
