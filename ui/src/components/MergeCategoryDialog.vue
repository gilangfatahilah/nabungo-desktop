<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { CategoryDto } from '@/ipc'
import { mergeTargets } from '@/lib/categories'
import { errorMessage, type Translate } from '@/lib/errors'
import { useCategoriesStore } from '@/stores/categories'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ source: CategoryDto }>()

const store = useCategoriesStore()
const { t } = useI18n()

const targets = computed(() => mergeTargets(store.list, props.source))
const targetId = ref<string>('')
const target = computed(() => targets.value.find((c) => c.id === targetId.value))
const error = ref<string | null>(null)
const busy = ref(false)

watch(open, (v) => {
  if (v) {
    targetId.value = ''
    error.value = null
  }
})

async function submit() {
  if (busy.value || !target.value) return
  busy.value = true
  error.value = null
  try {
    await store.mergeCategory(props.source.id, target.value.id)
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
        <DialogTitle>{{ t('categories.mergeTitle') }}: {{ source.name }}</DialogTitle>
        <DialogDescription class="sr-only">{{ t('categories.mergeTitle') }}</DialogDescription>
      </DialogHeader>
      <div class="flex flex-col gap-2">
        <Label id="merge-target-label">{{ t('categories.mergeTarget') }}</Label>
        <Select v-model="targetId">
          <SelectTrigger data-testid="merge-target" aria-labelledby="merge-target-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="c in targets" :key="c.id" :value="c.id">{{ c.name }}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <p v-if="target" class="text-sm">
        {{ t('categories.mergeBody', { source: source.name, target: target.name }) }}
      </p>
      <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
      <DialogFooter>
        <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
        <Button type="button" variant="destructive" :disabled="!target || busy" @click="submit">
          {{ t('categories.merge') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
