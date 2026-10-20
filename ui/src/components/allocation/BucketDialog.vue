<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import type { BucketDto } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'
import { formatBp, parsePercent } from '@/lib/percent'
import { useAllocationStore } from '@/stores/allocation'
import { useSettingsStore } from '@/stores/settings'

const open = defineModel<boolean>('open', { required: true })
// `parent` = induk bucket (baru maupun yang diubah); induk tidak bisa dipindah dari dialog ini.
const props = defineProps<{ bucket?: BucketDto; parent?: BucketDto | null }>()

const store = useAllocationStore()
const settings = useSettingsStore()
const { t } = useI18n()

const name = ref('')
const target = ref('')
const error = ref<string | null>(null)
const busy = ref(false)

const editing = computed(() => props.bucket !== undefined)

watch(
  open,
  (v) => {
    if (!v) return
    name.value = props.bucket?.name ?? ''
    target.value = props.bucket ? formatBp(props.bucket.targetBp, settings.locale).replace('%', '') : ''
    error.value = null
  },
  { immediate: true },
)

async function submit() {
  if (busy.value) return
  // Cek cepat di UI; validasi sebenarnya tetap di Rust.
  if (name.value.trim() === '') {
    error.value = t('errors.name_empty')
    return
  }
  const targetBp = parsePercent(target.value)
  if (targetBp === null) {
    error.value = t('allocation.percentInvalid')
    return
  }
  busy.value = true
  error.value = null
  const fields = { name: name.value, parentId: props.parent?.id ?? null, targetBp, color: props.bucket?.color ?? null }
  try {
    if (props.bucket) await store.updateBucket(props.bucket.id, fields)
    else await store.createBucket(fields)
    open.value = false
  } catch (e) {
    error.value = errorMessage(e, t as unknown as Translate)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ editing ? t('allocation.editBucket') : parent ? t('allocation.newChild') : t('allocation.newBucket') }}</DialogTitle>
        <DialogDescription>
          {{ parent ? t('allocation.targetChildHint', { name: parent.name }) : t('allocation.targetHint') }}
        </DialogDescription>
      </DialogHeader>
      <form data-testid="bucket-form" class="flex flex-col gap-4" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit">
        <div class="flex flex-col gap-2">
          <Label for="bucket-name">{{ t('allocation.name') }}</Label>
          <Input id="bucket-name" v-model="name" maxlength="80" autofocus :aria-invalid="error ? true : undefined" />
        </div>
        <div class="flex flex-col gap-2">
          <Label for="bucket-target">{{ t('allocation.target') }}</Label>
          <div class="flex items-center gap-2">
            <Input id="bucket-target" v-model="target" inputmode="decimal" class="num w-24 text-right" :aria-invalid="error ? true : undefined" />
            <span class="text-muted-foreground">%</span>
          </div>
        </div>
        <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
          <Button type="submit" :disabled="busy">
            {{ t('actions.save') }} <kbd class="ml-1 text-xs opacity-70">Ctrl+Enter</kbd>
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
