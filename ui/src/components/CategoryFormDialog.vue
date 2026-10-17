<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import IconColorPicker from '@/components/IconColorPicker.vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { CategoryDto, CategoryKindDto } from '@/ipc'
import { buildTree } from '@/lib/categories'
import { errorMessage, type Translate } from '@/lib/errors'
import { useCategoriesStore } from '@/stores/categories'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ kind: CategoryKindDto; category?: CategoryDto; parentId?: string | null }>()

const store = useCategoriesStore()
const { t } = useI18n()

// Select tidak menerima value kosong: pakai penanda untuk "Tanpa induk".
const NO_PARENT = '__none__'

const name = ref('')
const parent = ref(NO_PARENT)
const icon = ref<string | null>(null)
const color = ref<string | null>(null)
const error = ref<string | null>(null)
const busy = ref(false)

const editing = computed(() => props.category !== undefined)
// Pohon 2 level: hanya root aktif sejenis (selain dirinya) yang bisa jadi induk.
const parents = computed(() =>
  buildTree(store.list, props.kind, { archived: false })
    .map((n) => n.category)
    .filter((c) => c.id !== props.category?.id),
)
const hasChildren = computed(() => !!props.category && store.list.some((c) => c.parentId === props.category!.id))

watch(
  open,
  (v) => {
    if (!v) return
    const c = props.category
    name.value = c?.name ?? ''
    parent.value = (c ? c.parentId : props.parentId) ?? NO_PARENT
    icon.value = c?.icon ?? null
    color.value = c?.color ?? null
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
  const fields = {
    name: name.value,
    parentId: parent.value === NO_PARENT ? null : parent.value,
    icon: icon.value,
    color: color.value,
  }
  try {
    if (props.category) await store.updateCategory(props.category.id, fields)
    else await store.createCategory(props.kind, fields)
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
        <DialogTitle>{{ editing ? t('categories.edit') : t('categories.new') }}</DialogTitle>
        <DialogDescription>{{ t(`categories.${kind}`) }}</DialogDescription>
      </DialogHeader>
      <form class="flex flex-col gap-4" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit">
        <div class="flex flex-col gap-2">
          <Label for="cat-name">{{ t('categories.name') }}</Label>
          <Input id="cat-name" v-model="name" maxlength="80" autofocus :aria-invalid="error ? true : undefined" />
        </div>
        <div class="flex flex-col gap-2">
          <Label id="cat-parent">{{ t('categories.parent') }}</Label>
          <Select v-model="parent" :disabled="hasChildren">
            <SelectTrigger aria-labelledby="cat-parent" class="w-full" :disabled="hasChildren"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem :value="NO_PARENT">{{ t('categories.noParent') }}</SelectItem>
              <SelectItem v-for="c in parents" :key="c.id" :value="c.id">{{ c.name }}</SelectItem>
            </SelectContent>
          </Select>
          <p v-if="hasChildren" class="text-xs text-muted-foreground">{{ t('categories.parentLocked') }}</p>
        </div>
        <IconColorPicker v-model:icon="icon" v-model:color="color" />
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
