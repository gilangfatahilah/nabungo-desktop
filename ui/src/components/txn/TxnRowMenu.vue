<script setup lang="ts">
import { EllipsisIcon } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuShortcut, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import type { TxnRowDto } from '@/ipc'
import { useTxnRowActions } from '@/composables/useTxnRowActions'
import { isEditable } from '@/lib/txnForm'

// Menu `⋯` baris transaksi; tampil saat baris di-hover/difokus (baris induk memakai `group/row`).
const props = defineProps<{ row: TxnRowDto }>()

const { t } = useI18n()
const { edit, duplicate, makeRecurring, remove } = useTxnRowActions()
</script>

<template>
  <DropdownMenu>
    <DropdownMenuTrigger as-child>
      <Button
        variant="ghost" size="icon" class="size-8 opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100"
        data-testid="txn-menu" :aria-label="t('actions.more')" tabindex="-1" @click.stop
      >
        <EllipsisIcon />
      </Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end">
      <DropdownMenuItem data-testid="txn-menu-edit" :disabled="!isEditable(props.row.kind)" @select="edit(props.row)">
        {{ t('actions.edit') }}<DropdownMenuShortcut>Enter</DropdownMenuShortcut>
      </DropdownMenuItem>
      <DropdownMenuItem data-testid="txn-menu-duplicate" :disabled="!isEditable(props.row.kind)" @select="duplicate(props.row)">
        {{ t('actions.duplicate') }}<DropdownMenuShortcut>Ctrl+D</DropdownMenuShortcut>
      </DropdownMenuItem>
      <DropdownMenuItem data-testid="txn-menu-recurring" :disabled="!isEditable(props.row.kind)" @select="makeRecurring(props.row)">
        {{ t('scheduled.makeRecurring') }}
      </DropdownMenuItem>
      <DropdownMenuItem data-testid="txn-menu-delete" variant="destructive" @select="remove(props.row)">
        {{ t('actions.delete') }}<DropdownMenuShortcut>Del</DropdownMenuShortcut>
      </DropdownMenuItem>
    </DropdownMenuContent>
  </DropdownMenu>
</template>
