<script setup lang="ts">
import { isTauri } from '@tauri-apps/api/core'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import AppLogo from '@/components/AppLogo.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  inspectBackupSource, IpcFailure, listBackups, pickBackupFile, recoverFromBackup, recoveryStartFresh, recoveryStatus,
  type BackupDto, type BackupSecretDto, type BackupSourceDto, type RecoveryStatusDto,
} from '@/ipc'
import { formatTimestamp } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { reloadApp } from '@/lib/reload'
import { useSecurityStore } from '@/stores/security'
import { useSettingsStore } from '@/stores/settings'

const { t } = useI18n()
const tr = t as unknown as Translate
const router = useRouter()
const security = useSecurityStore()
const settings = useSettingsStore()

const status = ref<RecoveryStatusDto | null>(null)
const backups = ref<BackupDto[]>([])
const busy = ref(false)

/** Sumber yang dipilih; `needsKey` → form rahasia ditampilkan sebelum konfirmasi. */
const pending = ref<{ source: BackupSourceDto; needsKey: boolean } | null>(null)
const mode = ref<'password' | 'recoveryKey'>('recoveryKey')
const password = ref('')
const recoveryKey = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const confirmOpen = ref(false)
const freshOpen = ref(false)

// Error menempel di field yang salah (docs/design/design.md §7.3).
type Field = 'password' | 'recovery' | 'new' | 'confirm'
const error = ref<string | null>(null)
const errorField = ref<Field | null>(null)
const FIELD_IDS: Record<Field, string> = {
  password: 'recovery-password', recovery: 'recovery-key', new: 'recovery-new-password', confirm: 'recovery-confirm-password',
}

watch([password, recoveryKey, newPassword, confirmPassword, mode], () => {
  error.value = null
  errorField.value = null
})

const title = computed(() => {
  if (status.value?.reason === 'tooNew') return t('recovery.titleTooNew')
  if (status.value?.reason === 'unreadable') return t('recovery.titleUnreadable')
  return t('recovery.titleManual')
})
const body = computed(() => {
  if (status.value?.reason === 'tooNew') return t('recovery.bodyTooNew')
  if (status.value?.reason === 'unreadable') return t('recovery.bodyUnreadable')
  return t('recovery.bodyManual')
})
const keptNote = computed(() =>
  status.value?.dataDir ? t('recovery.keptNote', { dir: status.value.dataDir }) : t('recovery.keptNoteNoDir'),
)
/** Dibuka dari layar kunci (bukan mode pemulihan startup). */
const fromUnlock = computed(() => status.value !== null && !status.value.active)

onMounted(async () => {
  await withErrorToast(async () => {
    status.value = await recoveryStatus()
    backups.value = await listBackups()
  })
})

async function run(fn: () => Promise<unknown>) {
  busy.value = true
  try {
    await withErrorToast(fn)
  } finally {
    busy.value = false
  }
}

async function choose(source: BackupSourceDto) {
  const { needsKey } = await inspectBackupSource(source)
  pending.value = { source, needsKey }
  password.value = recoveryKey.value = newPassword.value = confirmPassword.value = ''
  mode.value = status.value?.hasKeyfile ? 'password' : 'recoveryKey'
  if (!needsKey) {
    confirmOpen.value = true
    return
  }
  await nextTick()
  document.getElementById(FIELD_IDS[mode.value === 'password' ? 'password' : 'recovery'])?.focus()
}

const chooseNamed = (b: BackupDto) => run(() => choose({ kind: 'named', fileName: b.fileName }))
const chooseFile = () =>
  run(async () => {
    const path = await pickBackupFile()
    if (path) await choose({ kind: 'path', path })
  })

function fail(field: Field, message: string) {
  error.value = message
  errorField.value = field
  document.getElementById(FIELD_IDS[field])?.focus()
}

/** Validasi lokal form rahasia, lalu konfirmasi destruktif. */
function submitSecret() {
  if (mode.value === 'password' && password.value === '') return fail('password', t('security.passwordRequired'))
  if (mode.value === 'recoveryKey') {
    if (recoveryKey.value.trim() === '') return fail('recovery', t('security.recoveryRequired'))
    if (newPassword.value !== confirmPassword.value) return fail('confirm', t('security.passwordMismatch'))
  }
  confirmOpen.value = true
}

function secret(): BackupSecretDto {
  if (!pending.value?.needsKey) return { kind: 'none' }
  if (mode.value === 'password') return { kind: 'password', password: password.value }
  return { kind: 'recoveryKey', recoveryKey: recoveryKey.value, newPassword: newPassword.value }
}

/** Kode error backend → field yang ditandai; `null` = toast. */
function fieldFor(code: string): Field | null {
  if (!pending.value?.needsKey) return null
  if (code === 'wrong_password' || code === 'key_file_invalid') return mode.value === 'password' ? 'password' : null
  if (code === 'invalid_recovery_key' || code === 'backup_key_required') return mode.value === 'password' ? 'password' : 'recovery'
  if (code === 'password_too_short') return mode.value === 'recoveryKey' ? 'new' : null
  return null
}

/** DB baru terbuka: muat ulang status & pengaturan, lalu pindah layar. */
async function leave(to: '/' | '/onboarding') {
  await withErrorToast(() => security.refresh())
  await withErrorToast(() => settings.load())
  await router.replace(to)
  // Di Tauri muat ulang UI agar tidak ada state dari sesi pemulihan; backend browser in-memory.
  if (isTauri()) reloadApp()
}

async function onConfirm() {
  const target = pending.value
  if (!target) return
  busy.value = true
  try {
    await recoverFromBackup(target.source, secret())
  } catch (e) {
    busy.value = false
    const field = e instanceof IpcFailure ? fieldFor(e.error.code) : null
    if (field) fail(field, errorMessage(e, tr))
    else toast.error(errorMessage(e, tr))
    return
  }
  toast.success(t('recovery.restored'))
  await leave('/')
  busy.value = false
}

function onFresh() {
  void run(async () => {
    await recoveryStartFresh()
    await leave('/onboarding')
  })
}

const when = (b: BackupDto) => formatTimestamp(b.createdAt, settings.locale)
const describedBy = (field: Field) => (errorField.value === field ? 'recovery-error' : undefined)
const invalid = (field: Field) => (errorField.value === field ? 'true' : undefined)
</script>

<template>
  <!-- Layar `bare` satu kolom di tengah, seperti layar kunci (docs/design/design.md §8.9). -->
  <main class="mx-auto flex min-h-screen w-full max-w-[560px] flex-col justify-center gap-8 px-4 py-10">
    <header class="flex flex-col gap-6">
      <AppLogo />
      <div>
        <h1 class="text-title font-semibold tracking-tight">{{ title }}</h1>
        <p class="mt-1 text-meta text-muted-foreground">{{ body }}</p>
      </div>
      <p v-if="status?.reason === 'tooNew'" class="text-meta text-destructive" role="status">{{ t('recovery.tooNewWarning') }}</p>
      <p class="text-meta text-muted-foreground">{{ keptNote }}</p>
    </header>

    <section class="flex flex-col gap-3" :aria-label="t('recovery.backups')">
      <h2 class="font-medium">{{ t('recovery.backups') }}</h2>
      <p v-if="backups.length === 0" class="text-meta text-muted-foreground">{{ t('recovery.empty') }}</p>
      <div v-else class="flex flex-col" role="list">
        <div
          v-for="b in backups" :key="b.fileName" role="listitem" data-testid="recovery-backup-row"
          class="flex h-12 items-center gap-3 border-b px-2"
        >
          <span class="flex-1" :title="b.fileName">{{ when(b) }}</span>
          <Badge variant="secondary">{{ t(`settings.backup.kinds.${b.kind}`) }}</Badge>
          <Button variant="ghost" size="sm" :disabled="busy" @click="chooseNamed(b)">{{ t('recovery.restore') }}</Button>
        </div>
      </div>
      <div>
        <Button variant="outline" :disabled="busy" @click="chooseFile">{{ t('recovery.pickFile') }}</Button>
      </div>
    </section>

    <form
      v-if="pending?.needsKey" data-testid="recovery-secret-form" class="flex flex-col gap-4 rounded-lg border p-4"
      novalidate @submit.prevent="submitSecret"
    >
      <div>
        <h2 class="font-medium">{{ t('recovery.encryptedTitle') }}</h2>
        <p class="mt-1 text-meta text-muted-foreground">
          {{ status?.hasKeyfile ? t('recovery.encryptedBody') : t('recovery.encryptedBodyNoKeyfile') }}
        </p>
      </div>

      <div v-if="mode === 'password'" class="flex flex-col gap-2">
        <Label for="recovery-password">{{ t('security.password') }}</Label>
        <Input
          id="recovery-password" v-model="password" type="password" autocomplete="current-password"
          :aria-invalid="invalid('password')" :aria-describedby="describedBy('password')"
        />
        <p v-if="errorField === 'password'" id="recovery-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
      </div>

      <template v-else>
        <div class="flex flex-col gap-2">
          <Label for="recovery-key">{{ t('security.recoveryKey') }}</Label>
          <Input
            id="recovery-key" v-model="recoveryKey" class="font-mono uppercase" autocomplete="off" spellcheck="false"
            placeholder="XXXX-XXXX-XXXX-…" :aria-invalid="invalid('recovery')" :aria-describedby="describedBy('recovery')"
          />
          <p v-if="errorField === 'recovery'" id="recovery-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
        </div>
        <div class="flex flex-col gap-2">
          <Label for="recovery-new-password">{{ t('security.newPassword') }}</Label>
          <Input
            id="recovery-new-password" v-model="newPassword" type="password" autocomplete="new-password"
            :aria-invalid="invalid('new')" :aria-describedby="describedBy('new')"
          />
          <p v-if="errorField === 'new'" id="recovery-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
        </div>
        <div class="flex flex-col gap-2">
          <Label for="recovery-confirm-password">{{ t('security.confirmPassword') }}</Label>
          <Input
            id="recovery-confirm-password" v-model="confirmPassword" type="password" autocomplete="new-password"
            :aria-invalid="invalid('confirm')" :aria-describedby="describedBy('confirm')"
          />
          <p v-if="errorField === 'confirm'" id="recovery-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
        </div>
      </template>

      <div class="flex flex-wrap gap-2">
        <Button type="submit" :disabled="busy">{{ t('recovery.continue') }}</Button>
        <Button
          v-if="status?.hasKeyfile" type="button" variant="ghost"
          @click="mode = mode === 'password' ? 'recoveryKey' : 'password'"
        >
          {{ mode === 'password' ? t('recovery.useRecoveryKey') : t('recovery.usePassword') }}
        </Button>
      </div>
    </form>

    <section class="flex flex-col gap-2 border-t pt-6">
      <h2 class="font-medium">{{ t('recovery.freshTitle') }}</h2>
      <p class="text-meta text-muted-foreground">{{ t('recovery.freshBody') }}</p>
      <div class="flex flex-wrap gap-2">
        <Button variant="outline" :disabled="busy" @click="freshOpen = true">{{ t('recovery.fresh') }}</Button>
        <Button v-if="fromUnlock" variant="ghost" @click="router.replace({ name: 'unlock' })">{{ t('recovery.backToUnlock') }}</Button>
      </div>
    </section>

    <ConfirmDialog
      v-model:open="confirmOpen" destructive :title="t('recovery.confirmTitle')" :description="t('recovery.confirmBody')"
      :confirm-label="t('recovery.confirm')" @confirm="onConfirm"
    />
    <ConfirmDialog
      v-model:open="freshOpen" destructive :title="t('recovery.freshConfirmTitle')" :description="t('recovery.freshBody')"
      :confirm-label="t('recovery.freshConfirm')" @confirm="onFresh"
    />
  </main>
</template>
