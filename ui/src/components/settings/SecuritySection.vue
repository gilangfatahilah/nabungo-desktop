<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { toast } from 'vue-sonner'
import EncryptionFields, { type EncryptionInput } from '@/components/EncryptionFields.vue'
import RecoveryKeyDialog from '@/components/RecoveryKeyDialog.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { IpcFailure } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { AUTO_LOCK_CHOICES, MIN_PASSWORD_CHARS } from '@/lib/security'
import { useSecurityStore } from '@/stores/security'

const security = useSecurityStore()
const router = useRouter()
const { t } = useI18n()
const tr = t as unknown as Translate

const busy = ref(false)
const codeOf = (e: unknown) => (e instanceof IpcFailure ? e.error.code : undefined)

onMounted(() => withErrorToast(() => security.refresh()))

// --- Aktifkan ---
const enableOpen = ref(false)
const enableInput = ref<EncryptionInput>({ password: '', confirm: '', acknowledged: false })
const enableValid = ref(false)
const enableSubmitted = ref(false)
const enableError = ref<string | null>(null)
const recoveryKey = ref('')
const recoveryOpen = ref(false)

function openEnable() {
  enableInput.value = { password: '', confirm: '', acknowledged: false }
  enableSubmitted.value = false
  enableError.value = null
  enableOpen.value = true
}
watch(() => enableInput.value.password, () => (enableError.value = null))

async function submitEnable() {
  if (busy.value) return
  enableSubmitted.value = true
  if (!enableValid.value) return
  busy.value = true
  try {
    recoveryKey.value = await security.enable(enableInput.value.password)
    enableOpen.value = false
    recoveryOpen.value = true
  } catch (e) {
    if (codeOf(e) === 'password_too_short') enableError.value = errorMessage(e, tr)
    else toast.error(errorMessage(e, tr))
  } finally {
    busy.value = false
  }
}

function onRecoveryDone() {
  recoveryKey.value = ''
  toast.success(t('security.enabled'))
}

// --- Kunci otomatis & kunci sekarang ---
function onAutoLock(v: unknown) {
  const n = Number(v)
  if ((AUTO_LOCK_CHOICES as readonly number[]).includes(n)) void withErrorToast(() => security.setAutoLock(n))
}

function lockNow() {
  void withErrorToast(async () => {
    await security.lock()
    await security.showLockScreen(router)
  })
}

// --- Ganti password ---
const cpOpen = ref(false)
const cp = ref({ old: '', next: '', confirm: '' })
const cpErrors = ref<{ old?: string; next?: string; confirm?: string }>({})
watch(cp, () => (cpErrors.value = {}), { deep: true })

function openChangePassword() {
  cp.value = { old: '', next: '', confirm: '' }
  cpOpen.value = true
}

async function submitChangePassword() {
  if (busy.value) return
  if (cp.value.old === '') return void (cpErrors.value = { old: t('security.passwordRequired') })
  if ([...cp.value.next].length < MIN_PASSWORD_CHARS) return void (cpErrors.value = { next: t('errors.password_too_short') })
  if (cp.value.next !== cp.value.confirm) return void (cpErrors.value = { confirm: t('security.passwordMismatch') })
  busy.value = true
  try {
    await security.changePassword(cp.value.old, cp.value.next)
    cpOpen.value = false
    toast.success(t('security.passwordChanged'))
  } catch (e) {
    const code = codeOf(e)
    if (code === 'wrong_password' || code === 'key_file_invalid') cpErrors.value = { old: errorMessage(e, tr) }
    else if (code === 'password_too_short') cpErrors.value = { next: errorMessage(e, tr) }
    else toast.error(errorMessage(e, tr))
  } finally {
    busy.value = false
  }
}

// --- Matikan ---
const disableOpen = ref(false)
const disablePassword = ref('')
const disableError = ref<string | null>(null)
watch(disablePassword, () => (disableError.value = null))

function openDisable() {
  disablePassword.value = ''
  disableOpen.value = true
}

async function submitDisable() {
  if (busy.value) return
  if (disablePassword.value === '') return void (disableError.value = t('security.passwordRequired'))
  busy.value = true
  try {
    await security.disable(disablePassword.value)
    disableOpen.value = false
    toast.success(t('security.disabled'))
  } catch (e) {
    const code = codeOf(e)
    if (code === 'wrong_password' || code === 'key_file_invalid') disableError.value = errorMessage(e, tr)
    else toast.error(errorMessage(e, tr))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <template v-if="!security.encrypted">
      <p class="max-w-prose text-muted-foreground">{{ t('security.intro') }}</p>
      <div>
        <Button :disabled="busy" @click="openEnable">{{ t('security.enable') }}</Button>
      </div>
    </template>

    <template v-else>
      <div class="flex items-center gap-3">
        <Badge>{{ t('security.encryptedBadge') }}</Badge>
        <p class="text-muted-foreground">{{ t('security.encryptedIntro') }}</p>
      </div>
      <div class="flex max-w-md flex-col gap-2">
        <label id="auto-lock-label" class="text-sm font-medium">{{ t('security.autoLock') }}</label>
        <Select :model-value="String(security.autoLockMinutes)" @update:model-value="onAutoLock">
          <SelectTrigger aria-labelledby="auto-lock-label" aria-describedby="auto-lock-hint" class="w-full" data-testid="auto-lock">
            <SelectValue>{{ t('security.minutes', { n: security.autoLockMinutes }) }}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="m in AUTO_LOCK_CHOICES" :key="m" :value="String(m)">{{ t('security.minutes', { n: m }) }}</SelectItem>
          </SelectContent>
        </Select>
        <p id="auto-lock-hint" class="text-xs text-muted-foreground">{{ t('security.autoLockHint') }}</p>
      </div>
      <div class="flex flex-wrap gap-2">
        <Button :disabled="busy" @click="lockNow">{{ t('security.lockNow') }}</Button>
        <Button variant="outline" :disabled="busy" @click="openChangePassword">{{ t('security.changePassword') }}</Button>
        <Button variant="outline" :disabled="busy" @click="openDisable">{{ t('security.disable') }}</Button>
      </div>
    </template>

    <Dialog v-model:open="enableOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{{ t('security.enableTitle') }}</DialogTitle>
          <DialogDescription>{{ t('security.enableBody') }}</DialogDescription>
        </DialogHeader>
        <form data-testid="enable-form" class="flex flex-col gap-6" novalidate @submit.prevent="submitEnable">
          <EncryptionFields
            v-model="enableInput" :show-errors="enableSubmitted" :password-error="enableError"
            @valid="enableValid = $event"
          />
          <DialogFooter>
            <Button type="button" variant="ghost" @click="enableOpen = false">{{ t('actions.cancel') }}</Button>
            <Button type="submit" :disabled="busy">{{ t('security.enable') }}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>

    <RecoveryKeyDialog v-model:open="recoveryOpen" :recovery-key="recoveryKey" @done="onRecoveryDone" />

    <Dialog v-model:open="cpOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{{ t('security.changePassword') }}</DialogTitle>
        </DialogHeader>
        <form data-testid="change-password-form" class="flex flex-col gap-4" novalidate @submit.prevent="submitChangePassword">
          <div class="flex flex-col gap-2">
            <Label for="cp-old">{{ t('security.currentPassword') }}</Label>
            <Input
              id="cp-old" v-model="cp.old" type="password" autocomplete="current-password"
              :aria-invalid="cpErrors.old ? 'true' : undefined" :aria-describedby="cpErrors.old ? 'cp-old-error' : undefined"
            />
            <p v-if="cpErrors.old" id="cp-old-error" class="text-meta text-destructive" role="alert">{{ cpErrors.old }}</p>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="cp-new">{{ t('security.newPassword') }}</Label>
            <Input
              id="cp-new" v-model="cp.next" type="password" autocomplete="new-password"
              :aria-invalid="cpErrors.next ? 'true' : undefined" aria-describedby="cp-new-help"
            />
            <p v-if="cpErrors.next" id="cp-new-help" class="text-meta text-destructive" role="alert">{{ cpErrors.next }}</p>
            <p v-else id="cp-new-help" class="text-meta text-muted-foreground">{{ t('security.passwordHint') }}</p>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="cp-confirm">{{ t('security.confirmPassword') }}</Label>
            <Input
              id="cp-confirm" v-model="cp.confirm" type="password" autocomplete="new-password"
              :aria-invalid="cpErrors.confirm ? 'true' : undefined" :aria-describedby="cpErrors.confirm ? 'cp-confirm-error' : undefined"
            />
            <p v-if="cpErrors.confirm" id="cp-confirm-error" class="text-meta text-destructive" role="alert">{{ cpErrors.confirm }}</p>
          </div>
          <DialogFooter>
            <Button type="button" variant="ghost" @click="cpOpen = false">{{ t('actions.cancel') }}</Button>
            <Button type="submit" :disabled="busy">{{ t('security.savePassword') }}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>

    <!-- Konfirmasi destruktif dengan field password; ConfirmDialog tidak punya slot field (§8.9). -->
    <Dialog v-model:open="disableOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{{ t('security.disableTitle') }}</DialogTitle>
          <DialogDescription>{{ t('security.disableBody') }}</DialogDescription>
        </DialogHeader>
        <form data-testid="disable-form" class="flex flex-col gap-4" novalidate @submit.prevent="submitDisable">
          <div class="flex flex-col gap-2">
            <Label for="disable-password">{{ t('security.password') }}</Label>
            <Input
              id="disable-password" v-model="disablePassword" type="password" autocomplete="current-password"
              :aria-invalid="disableError ? 'true' : undefined" :aria-describedby="disableError ? 'disable-error' : undefined"
            />
            <p v-if="disableError" id="disable-error" class="text-meta text-destructive" role="alert">{{ disableError }}</p>
          </div>
          <DialogFooter>
            <Button type="button" variant="ghost" @click="disableOpen = false">{{ t('actions.cancel') }}</Button>
            <Button type="submit" variant="destructive" :disabled="busy">{{ t('security.disable') }}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  </div>
</template>
