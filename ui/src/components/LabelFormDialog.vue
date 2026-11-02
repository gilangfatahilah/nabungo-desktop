<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import CategorySelect from '@/components/CategorySelect.vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { errorMessage, type Translate } from '@/lib/errors'
import type { Label as LabelDto, LabelKind } from '@/lib/labels'
import { useLabelsStore } from '@/stores/labels'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ kind: LabelKind; label?: LabelDto }>()

const store = useLabelsStore()
const { t } = useI18n()

const name = ref('')
const category = ref<string | null>(null)
const error = ref<string | null>(null)
const busy = ref(false)

watch(
  open,
  (v) => {
    if (!v) return
    name.value = props.label?.name ?? ''
    category.value = props.label && 'defaultCategoryId' in props.label ? props.label.defaultCategoryId : null
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
  busy.value = true
  error.value = null
  try {
    const id = props.label?.id
    if (props.kind === 'tag') {
      if (id) await store.renameTag(id, name.value)
      else await store.createTag(name.value)
    } else if (id) {
      await store.updatePayee(id, name.value, category.value)
    } else {
      await store.createPayee(name.value, category.value)
    }
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
        <DialogTitle>
          {{ kind === 'tag' ? t(label ? 'labels.editTag' : 'labels.newTag') : t(label ? 'labels.editPayee' : 'labels.newPayee') }}
        </DialogTitle>
        <DialogDescription class="sr-only">{{ t(kind === 'tag' ? 'labels.tags' : 'labels.payees') }}</DialogDescription>
      </DialogHeader>
      <form data-testid="label-form" class="flex flex-col gap-4" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit">
        <div class="flex flex-col gap-2">
          <Label for="label-name">{{ t('labels.name') }}</Label>
          <Input id="label-name" v-model="name" maxlength="80" autofocus :aria-invalid="error ? true : undefined" />
        </div>
        <div v-if="kind === 'payee'" class="flex flex-col gap-2">
          <Label for="label-category">{{ t('labels.defaultCategory') }}</Label>
          <CategorySelect id="label-category" v-model="category" class="w-full" :none-label="t('labels.noDefault')" />
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
