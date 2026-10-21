<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { MIN_PASSWORD_CHARS } from '@/lib/security'

export interface EncryptionInput {
  password: string
  confirm: string
  acknowledged: boolean
}

const model = defineModel<EncryptionInput>({ required: true })
const props = withDefaults(
  defineProps<{
    /** Prefix id field, agar unik bila dipakai lebih dari sekali di satu layar. */
    idPrefix?: string
    /** Tampilkan error field (setelah pengguna mencoba mengirim form). */
    showErrors?: boolean
    /** Error dari backend untuk field password (mis. `password_too_short`). */
    passwordError?: string | null
  }>(),
  { idPrefix: 'enc', showErrors: false, passwordError: null },
)
const emit = defineEmits<{ valid: [boolean] }>()
const { t } = useI18n()

const id = (field: string) => `${props.idPrefix}-${field}`
const set = <K extends keyof EncryptionInput>(k: K, v: EncryptionInput[K]) => (model.value = { ...model.value, [k]: v })

// Rust menghitung panjang per karakter (code point), bukan per unit UTF-16.
const tooShort = computed(() => [...model.value.password].length < MIN_PASSWORD_CHARS)
const mismatch = computed(() => model.value.confirm !== model.value.password)

const errors = computed(() => ({
  password: props.passwordError ?? (props.showErrors && tooShort.value ? t('errors.password_too_short') : null),
  confirm: props.showErrors && !tooShort.value && mismatch.value ? t('security.passwordMismatch') : null,
  ack: props.showErrors && !model.value.acknowledged ? t('security.ackRequired') : null,
}))

watch(
  () => !tooShort.value && !mismatch.value && model.value.acknowledged,
  (v) => emit('valid', v),
  { immediate: true },
)
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-col gap-2">
      <Label :for="id('password')">{{ t('security.password') }}</Label>
      <Input
        :id="id('password')" :model-value="model.password" type="password" autocomplete="new-password"
        :aria-invalid="errors.password ? 'true' : undefined" :aria-describedby="id('password-help')"
        @update:model-value="set('password', String($event))"
      />
      <p v-if="errors.password" :id="id('password-help')" class="text-meta text-destructive" role="alert">{{ errors.password }}</p>
      <p v-else :id="id('password-help')" class="text-meta text-muted-foreground">{{ t('security.passwordHint') }}</p>
    </div>
    <div class="flex flex-col gap-2">
      <Label :for="id('confirm')">{{ t('security.confirmPassword') }}</Label>
      <Input
        :id="id('confirm')" :model-value="model.confirm" type="password" autocomplete="new-password"
        :aria-invalid="errors.confirm ? 'true' : undefined" :aria-describedby="errors.confirm ? id('confirm-error') : undefined"
        @update:model-value="set('confirm', String($event))"
      />
      <p v-if="errors.confirm" :id="id('confirm-error')" class="text-meta text-destructive" role="alert">{{ errors.confirm }}</p>
    </div>
    <div class="flex flex-col gap-2">
      <div class="flex items-start gap-3">
        <Checkbox
          :id="id('ack')" :model-value="model.acknowledged" class="mt-0.5"
          :aria-invalid="errors.ack ? 'true' : undefined" :aria-describedby="errors.ack ? id('ack-error') : undefined"
          @update:model-value="set('acknowledged', $event === true)"
        />
        <Label :for="id('ack')" class="leading-snug font-normal">{{ t('security.warning') }}</Label>
      </div>
      <p v-if="errors.ack" :id="id('ack-error')" class="text-meta text-destructive" role="alert">{{ errors.ack }}</p>
    </div>
  </div>
</template>
