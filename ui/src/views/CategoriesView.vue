<script setup lang="ts">
import PageHeader from '@/components/PageHeader.vue'
import { ChevronDown, EllipsisVertical, Plus } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import CategoryFormDialog from '@/components/CategoryFormDialog.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import MergeCategoryDialog from '@/components/MergeCategoryDialog.vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { IpcFailure, type CategoryDto, type CategoryKindDto } from '@/ipc'
import { buildTree } from '@/lib/categories'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { withErrorToast } from '@/lib/notify'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'

const store = useCategoriesStore()
const { t } = useI18n()

const KINDS: CategoryKindDto[] = ['expense', 'income']
const tab = ref<CategoryKindDto>('expense')

const trees = computed(() => ({
  expense: buildTree(store.list, 'expense', { archived: false }),
  income: buildTree(store.list, 'income', { archived: false }),
}))
const archived = computed(() => store.list.filter((c) => c.archived).sort((a, b) => a.name.localeCompare(b.name)))
const hasChildren = (c: CategoryDto) => store.list.some((x) => x.parentId === c.id && !x.archived)

onMounted(() => {
  if (!store.loaded) void withErrorToast(() => store.load())
})

// --- form ---
const formOpen = ref(false)
const editing = ref<CategoryDto | undefined>()
const formParent = ref<string | null>(null)

function create(parentId: string | null = null) {
  editing.value = undefined
  formParent.value = parentId
  formOpen.value = true
}

function edit(c: CategoryDto) {
  editing.value = c
  formOpen.value = true
}

// --- gabung ---
const mergeOpen = ref(false)
const mergeSource = ref<CategoryDto | null>(null)

function merge(c: CategoryDto) {
  mergeSource.value = c
  mergeOpen.value = true
}

// --- konfirmasi ---
interface Pending {
  title: string
  description: string
  confirmLabel: string
  destructive: boolean
  secondaryLabel?: string
  run: () => Promise<unknown>
  secondary?: () => void
}
const pending = ref<Pending | null>(null)
const confirmOpen = ref(false)

function ask(p: Pending) {
  pending.value = p
  confirmOpen.value = true
}

function onConfirm() {
  const p = pending.value
  if (p) void withErrorToast(p.run)
}

function remove(c: CategoryDto) {
  ask({
    title: t('categories.deleteTitle'),
    description: t('categories.deleteBody', { name: c.name }),
    confirmLabel: t('actions.delete'),
    destructive: true,
    run: async () => {
      try {
        await store.deleteCategory(c.id)
      } catch (e) {
        // Kategori terpakai tidak bisa dihapus (acceptance "hapus aman"): tawarkan arsip atau gabung.
        if (!(e instanceof IpcFailure && e.error.code === 'in_use')) throw e
        ask({
          title: t('categories.inUseTitle'),
          description: t('categories.inUseBody'),
          confirmLabel: t('actions.archive'),
          destructive: false,
          secondaryLabel: c.archived ? undefined : t('categories.merge'),
          run: () => store.archiveCategory(c.id),
          secondary: () => merge(c),
        })
      }
    },
  })
}

function archive(c: CategoryDto) {
  if (!hasChildren(c)) {
    void withErrorToast(() => store.archiveCategory(c.id))
    return
  }
  ask({
    title: t('actions.archive'),
    description: t('categories.archiveParentBody', { name: c.name }),
    confirmLabel: t('actions.archive'),
    destructive: false,
    run: () => store.archiveCategory(c.id),
  })
}

function restore(c: CategoryDto) {
  void withErrorToast(() => store.unarchiveCategory(c.id))
}
</script>

<template>
  <PageHeader :title="t('nav.categories')">
    <template #actions>
      <Button @click="create()"><Plus /> {{ t('categories.new') }}</Button>
    </template>
  </PageHeader>
  <div class="flex flex-col gap-6">

    <Tabs v-model="tab">
      <TabsList>
        <TabsTrigger v-for="k in KINDS" :key="k" :value="k">{{ t(`categories.${k}`) }}</TabsTrigger>
      </TabsList>
      <TabsContent v-for="k in KINDS" :key="k" :value="k" class="rounded-lg border p-3">
        <ul class="flex flex-col">
          <template v-for="node in trees[k]" :key="node.category.id">
            <li v-for="(c, ci) in [node.category, ...node.children]" :key="c.id"
              :class="cn('flex items-center gap-2 rounded-md px-1 py-1.5 hover:bg-muted/60', ci > 0 && 'ml-8')">
              <span :class="cn('flex size-7 shrink-0 items-center justify-center rounded-md', colorClass(c.color))">
                <component :is="iconFor(c.icon)" class="size-4" />
              </span>
              <span class="min-w-0 flex-1 truncate" :class="ci === 0 && 'font-medium'">{{ c.name }}</span>
              <Button v-if="ci === 0" variant="ghost" size="icon-sm" :aria-label="t('categories.newSub')"
                :title="t('categories.newSub')" @click="create(c.id)">
                <Plus />
              </Button>
              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <Button variant="ghost" size="icon-sm" :aria-label="t('actions.more')" :data-testid="`category-menu-${c.id}`">
                    <EllipsisVertical />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem @select="edit(c)">{{ t('actions.edit') }}</DropdownMenuItem>
                  <DropdownMenuItem @select="merge(c)">{{ t('categories.merge') }}…</DropdownMenuItem>
                  <DropdownMenuItem @select="archive(c)">{{ t('actions.archive') }}</DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem variant="destructive" @select="remove(c)">{{ t('actions.delete') }}</DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </li>
          </template>
          <li v-if="!trees[k].length" class="px-2 py-2 text-sm text-muted-foreground">{{ t('categories.empty') }}</li>
        </ul>
      </TabsContent>
    </Tabs>

    <Collapsible v-if="archived.length" class="rounded-lg border p-3">
      <CollapsibleTrigger as-child>
        <Button variant="ghost" class="w-full justify-between">
          {{ t('categories.archived') }} ({{ archived.length }})
          <ChevronDown />
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <ul class="flex flex-col">
          <li v-for="c in archived" :key="c.id" class="flex items-center gap-2 px-1 py-1.5">
            <component :is="iconFor(c.icon)" class="size-4 text-muted-foreground" />
            <span class="flex-1 truncate">{{ c.name }}</span>
            <span class="text-xs text-muted-foreground">{{ t(`categories.${c.kind}`) }}</span>
            <Button variant="ghost" size="sm" @click="restore(c)">{{ t('actions.restore') }}</Button>
            <Button variant="ghost" size="sm" class="text-destructive" @click="remove(c)">{{ t('actions.delete') }}</Button>
          </li>
        </ul>
      </CollapsibleContent>
    </Collapsible>

    <CategoryFormDialog v-model:open="formOpen" :kind="editing?.kind ?? tab" :category="editing" :parent-id="formParent" />
    <MergeCategoryDialog v-if="mergeSource" v-model:open="mergeOpen" :source="mergeSource" />
    <ConfirmDialog
      v-if="pending"
      v-model:open="confirmOpen"
      :title="pending.title"
      :description="pending.description"
      :confirm-label="pending.confirmLabel"
      :destructive="pending.destructive"
      :secondary-label="pending.secondaryLabel"
      @confirm="onConfirm"
      @secondary="pending.secondary?.()"
    />
  </div>
</template>
