<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { errorMessage, type Translate } from '@/lib/errors'
import { labelText, type Label as LabelDto, type LabelKind } from '@/lib/labels'
import { useLabelsStore } from '@/stores/labels'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ kind: LabelKind; source: LabelDto }>()

const store = useLabelsStore()
const { t } = useI18n()

const targets = computed<LabelDto[]>(() =>
  (props.kind === 'tag' ? store.tags : store.payees).filter((x) => x.id !== props.source.id),
)
const targetId = ref<string>('')
const target = computed(() => targets.value.find((x) => x.id === targetId.value))
const error = ref<string | null>(null)
const busy = ref(false)
const text = (x: LabelDto) => labelText(props.kind, x.name)

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
    if (props.kind === 'tag') await store.mergeTag(props.source.id, target.value.id)
    else await store.mergePayee(props.source.id, target.value.id)
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
        <DialogTitle>{{ t('labels.mergeTitle', { name: text(source) }) }}</DialogTitle>
        <DialogDescription class="sr-only">{{ t('labels.mergeTarget') }}</DialogDescription>
      </DialogHeader>
      <div class="flex flex-col gap-2">
        <Label id="label-merge-target">{{ t('labels.mergeTarget') }}</Label>
        <Select v-model="targetId">
          <SelectTrigger data-testid="merge-target" aria-labelledby="label-merge-target" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="x in targets" :key="x.id" :value="x.id">{{ text(x) }}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <p v-if="target" class="text-sm">
        {{ t('labels.mergeBody', { source: text(source), target: text(target) }) }}
      </p>
      <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
      <DialogFooter>
        <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
        <Button type="button" variant="destructive" :disabled="!target || busy" @click="submit">
          {{ t('labels.merge') }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
