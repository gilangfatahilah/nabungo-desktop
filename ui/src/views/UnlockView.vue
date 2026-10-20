<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import AppLogo from '@/components/AppLogo.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { useSecurityStore } from '@/stores/security'
import { useSettingsStore } from '@/stores/settings'

const security = useSecurityStore()
const settings = useSettingsStore()
const router = useRouter()
const route = useRoute()
const { t } = useI18n()

const mode = ref<'password' | 'recovery'>('password')
const password = ref('')
const recoveryKey = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const error = ref<string | null>(null)
const busy = ref(false)

// Error menempel di field yang salah (docs/design/ui-guidelines.md §8.3).
const errorField = ref<'password' | 'recovery' | 'new' | 'confirm' | null>(null)

watch([password, recoveryKey, newPassword, confirmPassword, mode], () => {
  error.value = null
  errorField.value = null
})

const redirect = computed(() => {
  const r = route.query.redirect
  return typeof r === 'string' && r.startsWith('/') && !r.startsWith('/unlock') ? r : '/'
})

onMounted(async () => {
  await withErrorToast(() => security.refresh())
  // Tanpa key file, password tidak bisa dipakai sama sekali.
  if (security.encrypted && !security.hasKeyfile) mode.value = 'recovery'
  await focusFirst()
})

async function focusFirst() {
  await nextTick()
  document.getElementById(mode.value === 'password' ? 'unlock-password' : 'unlock-recovery')?.focus()
}

function switchMode(m: 'password' | 'recovery') {
  mode.value = m
  void focusFirst()
}

function fail(field: NonNullable<typeof errorField.value>, message: string) {
  error.value = message
  errorField.value = field
  const ids = { password: 'unlock-password', recovery: 'unlock-recovery', new: 'unlock-new-password', confirm: 'unlock-confirm-password' }
  document.getElementById(ids[field])?.focus()
}

/** Kode error backend → field yang ditandai. */
function fieldFor(code: string | undefined): NonNullable<typeof errorField.value> {
  if (mode.value === 'password') return 'password'
  return code === 'password_too_short' ? 'new' : 'recovery'
}

async function submit() {
  if (busy.value) return
  if (mode.value === 'password' && password.value === '') return fail('password', t('security.passwordRequired'))
  if (mode.value === 'recovery') {
    if (recoveryKey.value.trim() === '') return fail('recovery', t('security.recoveryRequired'))
    if (newPassword.value !== confirmPassword.value) return fail('confirm', t('security.passwordMismatch'))
  }
  busy.value = true
  try {
    if (mode.value === 'password') await security.unlock(password.value)
    else await security.unlockWithRecovery(recoveryKey.value, newPassword.value)
  } catch (e) {
    busy.value = false
    const code = (e as { error?: { code?: string } }).error?.code
    return fail(fieldFor(code), errorMessage(e, t as unknown as Translate))
  }
  password.value = ''
  newPassword.value = ''
  confirmPassword.value = ''
  // Pengaturan (tema, bahasa, periode) baru bisa dibaca setelah DB terbuka.
  await withErrorToast(() => settings.load())
  busy.value = false
  await router.replace(redirect.value)
}

const describedBy = (field: NonNullable<typeof errorField.value>) => (errorField.value === field ? 'unlock-error' : undefined)
const invalid = (field: NonNullable<typeof errorField.value>) => (errorField.value === field ? 'true' : undefined)
</script>

<template>
  <!-- Layar `bare` satu kolom 440px di tengah, seperti onboarding (docs/design/ui-guidelines.md §9.7). -->
  <main class="mx-auto flex min-h-screen w-full max-w-[440px] flex-col justify-center gap-8 px-4 py-10">
    <header class="flex flex-col gap-6">
      <AppLogo />
      <div>
        <h1 class="text-title font-semibold tracking-tight">
          {{ mode === 'password' ? t('security.lockedTitle') : t('security.recoveryTitle') }}
        </h1>
        <p class="mt-1 text-meta text-muted-foreground">
          {{ mode === 'password' ? t('security.lockedSubtitle') : t('security.recoverySubtitle') }}
        </p>
      </div>
      <p v-if="mode === 'recovery' && security.encrypted && !security.hasKeyfile" class="text-meta text-destructive" role="status">
        {{ t('security.keyfileMissing') }}
      </p>
    </header>

    <form class="flex flex-col gap-6" novalidate @submit.prevent="submit">
      <div v-if="mode === 'password'" class="flex flex-col gap-2">
        <Label for="unlock-password">{{ t('security.password') }}</Label>
        <Input
          id="unlock-password" v-model="password" type="password" autocomplete="current-password"
          :aria-invalid="invalid('password')" :aria-describedby="describedBy('password')"
        />
        <p v-if="errorField === 'password'" id="unlock-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
      </div>

      <template v-else>
        <div class="flex flex-col gap-2">
          <Label for="unlock-recovery">{{ t('security.recoveryKey') }}</Label>
          <Input
            id="unlock-recovery" v-model="recoveryKey" class="font-mono uppercase" autocomplete="off" spellcheck="false"
            placeholder="XXXX-XXXX-XXXX-…" :aria-invalid="invalid('recovery')" :aria-describedby="describedBy('recovery')"
          />
          <p v-if="errorField === 'recovery'" id="unlock-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
        </div>
        <div class="flex flex-col gap-2">
          <Label for="unlock-new-password">{{ t('security.newPassword') }}</Label>
          <Input
            id="unlock-new-password" v-model="newPassword" type="password" autocomplete="new-password"
            :aria-invalid="invalid('new')" :aria-describedby="describedBy('new')"
          />
          <p v-if="errorField === 'new'" id="unlock-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
        </div>
        <div class="flex flex-col gap-2">
          <Label for="unlock-confirm-password">{{ t('security.confirmPassword') }}</Label>
          <Input
            id="unlock-confirm-password" v-model="confirmPassword" type="password" autocomplete="new-password"
            :aria-invalid="invalid('confirm')" :aria-describedby="describedBy('confirm')"
          />
          <p v-if="errorField === 'confirm'" id="unlock-error" class="text-meta text-destructive" role="alert">{{ error }}</p>
        </div>
      </template>

      <div class="flex flex-col gap-3">
        <Button type="submit" size="lg" :disabled="busy">
          {{ mode === 'password' ? t('security.unlock') : t('security.recover') }}
        </Button>
        <Button
          v-if="mode === 'password'" type="button" variant="ghost" data-testid="unlock-use-recovery"
          @click="switchMode('recovery')"
        >
          {{ t('security.useRecovery') }}
        </Button>
        <Button
          v-else-if="security.hasKeyfile" type="button" variant="ghost" data-testid="unlock-use-password"
          @click="switchMode('password')"
        >
          {{ t('security.usePassword') }}
        </Button>
      </div>
    </form>
  </main>
</template>
