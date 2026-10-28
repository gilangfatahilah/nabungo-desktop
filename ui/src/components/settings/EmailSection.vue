<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import { Loader2Icon } from '@lucide/vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import * as ipc from '@/ipc'
import type { EmailStatusDto, EmailSyncDone } from '@/ipc'
import { formatTimestamp } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const { t } = useI18n()
const tr = t as unknown as Translate

const status = ref<EmailStatusDto | null>(null)
const connected = computed(() => status.value?.enabled === true)
/** Jumlah email yang sudah diproses sync yang sedang berjalan; `null` = tidak ada sync. */
const progress = ref<number | null>(null)
const syncing = ref(false)
const running = computed(() => syncing.value || progress.value !== null || status.value?.running === true)

const editing = ref(false)
const showForm = computed(() => status.value !== null && (!connected.value || editing.value))
const form = ref({ address: '', password: '', mailbox: 'Finance' })
const formError = ref<string | null>(null)
const busy = ref(false)
const disconnectOpen = ref(false)

const lastSync = computed(() =>
  status.value?.lastSyncAt
    ? t('email.lastSync', { when: formatTimestamp(status.value.lastSyncAt, settings.locale) })
    : t('email.neverSynced'),
)
const lastError = computed(() => (status.value?.lastError ? t(`errors.${status.value.lastError}`) : null))

async function refresh() {
  status.value = await ipc.emailStatus()
}

function resetForm() {
  form.value = { address: status.value?.address ?? '', password: '', mailbox: status.value?.mailbox ?? 'Finance' }
  formError.value = null
}

let off: (() => void) | null = null
onMounted(async () => {
  await withErrorToast(refresh)
  resetForm()
  off = await ipc.onEmailSync({
    progress: (p) => (progress.value = p.processed),
    done: onDone,
  })
})
onUnmounted(() => off?.())

/** Sync manual maupun terjadwal berakhir di sini; galat ditampilkan lewat `lastError`. */
function onDone(d: EmailSyncDone) {
  progress.value = null
  if (d.summary) toast.success(t('email.syncDone', { n: d.summary.newItems, m: d.summary.unparsed }))
  void withErrorToast(refresh)
}

function startEdit() {
  resetForm()
  editing.value = true
}

async function submit() {
  formError.value = null
  const password = form.value.password.trim() === '' ? null : form.value.password
  // Saat belum terhubung keychain kosong: password wajib.
  if (password === null && !connected.value) {
    formError.value = t('email.passwordRequired')
    return
  }
  busy.value = true
  try {
    await ipc.connectEmail(form.value.address, form.value.mailbox, password)
    editing.value = false
    await refresh()
    resetForm()
    toast.success(t('email.connectedToast'))
  } catch (e) {
    formError.value = errorMessage(e, tr)
  } finally {
    form.value.password = ''
    busy.value = false
  }
}

async function syncNow() {
  syncing.value = true
  try {
    await ipc.syncEmail()
  } catch (e) {
    toast.error(errorMessage(e, tr))
  } finally {
    syncing.value = false
  }
}

async function disconnect() {
  await withErrorToast(async () => {
    await ipc.disconnectEmail()
    editing.value = false
    await refresh()
    resetForm()
    toast.success(t('email.disconnectedToast'))
  })
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="max-w-prose text-muted-foreground">{{ t('email.intro') }}</p>

    <div v-if="connected && status" class="flex flex-col gap-3" data-testid="email-connected">
      <div class="flex flex-wrap items-center gap-3">
        <Badge>{{ t('email.connected') }}</Badge>
        <Badge v-if="status.paused" variant="destructive">{{ t('email.paused') }}</Badge>
        <span class="font-medium">{{ status.address }}</span>
        <span class="text-muted-foreground">· {{ status.mailbox }}</span>
      </div>
      <p class="text-muted-foreground" aria-live="polite">
        <template v-if="progress !== null">{{ t('email.syncing', { n: progress }) }}</template>
        <template v-else>{{ lastSync }}</template>
      </p>
      <p v-if="lastError" class="text-meta text-destructive" role="alert">{{ lastError }}</p>
      <div v-if="!editing" class="flex flex-wrap gap-2">
        <Button :disabled="running" @click="syncNow">
          <Loader2Icon v-if="running" class="size-4 animate-spin" aria-hidden="true" />
          {{ t('email.syncNow') }}
        </Button>
        <Button variant="outline" @click="startEdit">{{ t('email.edit') }}</Button>
        <Button variant="outline" @click="disconnectOpen = true">{{ t('email.disconnect') }}</Button>
      </div>
      <div class="flex gap-4">
        <RouterLink to="/inbox" class="text-meta text-primary underline-offset-4 hover:underline">{{ t('email.reviewInInbox') }}</RouterLink>
        <RouterLink to="/inbox?tab=rules" class="text-meta text-primary underline-offset-4 hover:underline">{{ t('emailRules.manageRules') }}</RouterLink>
      </div>
    </div>

    <template v-if="showForm">
      <div v-if="!connected" class="max-w-prose">
        <h3 class="mb-2 font-medium">{{ t('email.setupTitle') }}</h3>
        <ol class="list-decimal space-y-1 pl-5 text-muted-foreground">
          <li>{{ t('email.step1') }}</li>
          <li>{{ t('email.step2') }}</li>
          <li>{{ t('email.step3') }}</li>
        </ol>
      </div>
      <form class="flex max-w-md flex-col gap-4" novalidate @submit.prevent="submit">
        <div class="flex flex-col gap-2">
          <Label for="email-address">{{ t('email.address') }}</Label>
          <Input id="email-address" v-model="form.address" type="email" autocomplete="email" :disabled="busy" />
        </div>
        <div class="flex flex-col gap-2">
          <Label for="email-password">{{ t('email.password') }}</Label>
          <Input
            id="email-password"
            v-model="form.password"
            type="password"
            autocomplete="off"
            :disabled="busy"
            :aria-describedby="connected ? 'email-password-hint' : undefined"
          />
          <p v-if="connected" id="email-password-hint" class="text-xs text-muted-foreground">{{ t('email.passwordKeep') }}</p>
        </div>
        <div class="flex flex-col gap-2">
          <Label for="email-mailbox">{{ t('email.mailbox') }}</Label>
          <Input id="email-mailbox" v-model="form.mailbox" autocomplete="off" :disabled="busy" />
        </div>
        <p v-if="formError" class="text-meta text-destructive" role="alert">{{ formError }}</p>
        <div class="flex flex-wrap gap-2">
          <Button type="submit" :disabled="busy">
            <Loader2Icon v-if="busy" class="size-4 animate-spin" aria-hidden="true" />
            {{ connected ? t('email.save') : t('email.connect') }}
          </Button>
          <Button v-if="editing" type="button" variant="outline" :disabled="busy" @click="editing = false">
            {{ t('actions.cancel') }}
          </Button>
        </div>
      </form>
    </template>

    <ConfirmDialog
      v-model:open="disconnectOpen"
      :title="t('email.disconnectTitle')"
      :description="t('email.disconnectBody')"
      :confirm-label="t('email.disconnect')"
      destructive
      @confirm="disconnect"
    />
  </div>
</template>
