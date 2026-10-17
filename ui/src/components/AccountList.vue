<script setup lang="ts">
import { useSortable } from '@vueuse/integrations/useSortable'
import { EllipsisVertical, GripVertical } from '@lucide/vue'
import type { SortableEvent } from 'sortablejs'
import { useTemplateRef } from 'vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { AccountDto } from '@/ipc'
import { balanceTone, KIND_ICONS } from '@/lib/accounts'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { cn } from '@/lib/utils'

// Satu list per group; seret lintas list lewat `group: 'accounts'` milik Sortable.
const props = defineProps<{ groupId: string | null; accounts: AccountDto[]; archived?: boolean }>()
const emit = defineEmits<{
  move: [accountId: string, groupId: string | null, index: number]
  edit: [account: AccountDto]
  archive: [account: AccountDto]
  restore: [account: AccountDto]
  delete: [account: AccountDto]
}>()

const el = useTemplateRef<HTMLElement>('list')

function onEnd(e: SortableEvent) {
  const { item, from, to, oldIndex, newIndex } = e
  if (oldIndex === undefined || newIndex === undefined || (from === to && oldIndex === newIndex)) return
  // Kembalikan DOM ke posisi semula; Vue me-render ulang dari data setelah backend menyimpan urutan.
  item.remove()
  from.insertBefore(item, from.children[oldIndex] ?? null)
  const target = to.dataset.group || null
  emit('move', item.dataset.id!, target, newIndex)
}

if (!props.archived) {
  useSortable(el, [], { group: 'accounts', handle: '[data-drag]', animation: 150, onUpdate: () => {}, onEnd })
}
</script>

<template>
  <ul ref="list" :data-group="groupId ?? ''" class="flex min-h-12 flex-col">
    <li
      v-for="(a, i) in accounts"
      :key="a.id"
      :data-id="a.id"
      data-testid="account-row"
      class="group/row relative flex h-12 items-center gap-3 rounded-lg px-2 hover:bg-muted"
    >
      <!-- Pegangan seret di gutter kiri agar kolom ikon tetap sejajar dengan header grup. -->
      <span
        v-if="!archived" data-drag aria-hidden="true"
        class="absolute -left-4 cursor-grab text-muted-foreground opacity-0 group-hover/row:opacity-100"
      >
        <GripVertical class="size-4" />
      </span>
      <span :class="cn('flex size-7 shrink-0 items-center justify-center rounded-md', colorClass(a.color))">
        <component :is="iconFor(a.icon ?? KIND_ICONS[a.kind])" class="size-4" />
      </span>
      <span class="min-w-0 flex-1">
        <span class="block truncate">{{ a.name }}</span>
        <span class="block text-meta text-muted-foreground">
          {{ $t(`accountKinds.${a.kind}`) }}<template v-if="!a.includeInNetWorth"> · ∉ {{ $t('accounts.netWorth') }}</template>
        </span>
      </span>
      <MoneyText :amount="a.balance" :kind="balanceTone(a)" />
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button
            variant="ghost" size="icon" class="size-8 opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100"
            :aria-label="$t('actions.more')"
          >
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <template v-if="archived">
            <DropdownMenuItem @select="emit('restore', a)">{{ $t('actions.restore') }}</DropdownMenuItem>
          </template>
          <template v-else>
            <DropdownMenuItem @select="emit('edit', a)">{{ $t('actions.edit') }}</DropdownMenuItem>
            <DropdownMenuItem :disabled="i === 0" @select="emit('move', a.id, groupId, i - 1)">
              {{ $t('actions.moveUp') }}
            </DropdownMenuItem>
            <DropdownMenuItem :disabled="i === accounts.length - 1" @select="emit('move', a.id, groupId, i + 1)">
              {{ $t('actions.moveDown') }}
            </DropdownMenuItem>
            <DropdownMenuItem @select="emit('archive', a)">{{ $t('actions.archive') }}</DropdownMenuItem>
          </template>
          <DropdownMenuSeparator />
          <DropdownMenuItem variant="destructive" @select="emit('delete', a)">{{ $t('actions.delete') }}</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
    <li v-if="!accounts.length" class="flex h-12 items-center px-2 text-meta text-muted-foreground">{{ $t('accounts.empty') }}</li>
  </ul>
</template>
