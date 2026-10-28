<script setup lang="ts">
import { EllipsisVertical } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import EmptyState from '@/components/EmptyState.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Switch } from '@/components/ui/switch'
import type { EmailRuleDto } from '@/ipc'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'

// Daftar rule email capture urut prioritas (EML-04, docs/design/design.md §8.10). Akun/kategori kosong
// atau diarsipkan ditulis "Tanpa akun"/"Tanpa kategori" berwarna warning.
defineProps<{ rules: EmailRuleDto[]; busy: string | null }>()
defineEmits<{ edit: [EmailRuleDto]; duplicate: [EmailRuleDto]; delete: [EmailRuleDto]; toggle: [EmailRuleDto, boolean] }>()

const { t } = useI18n()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const reveal = 'opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100'

const accountName = (id: string | null) => accounts.overview.accounts.find((a) => a.id === id && !a.archived)?.name ?? null
const categoryName = (id: string | null) => categories.list.find((c) => c.id === id && !c.archived)?.name ?? null

function target(r: EmailRuleDto) {
  return r.kind === 'transfer_out' || r.kind === 'transfer_in'
    ? { name: accountName(r.counterAccountId), missing: t('emailRules.noAccount') }
    : { name: categoryName(r.defaultCategoryId), missing: t('emailRules.noCategory') }
}
</script>

<template>
  <EmptyState v-if="!rules.length" :title="t('emailRules.empty')" />
  <ul v-else class="-mx-2 flex flex-col">
    <li
      v-for="r in rules" :key="r.id" data-testid="rule-row" :data-rule-id="r.id"
      class="group/row flex min-h-12 items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-muted"
    >
      <div class="min-w-0 flex-1">
        <div class="flex min-w-0 items-center gap-2">
          <button
            type="button" data-part="title"
            class="truncate rounded-sm text-left font-medium hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            @click="$emit('edit', r)"
          >{{ r.name }}</button>
          <Badge v-if="!r.enabled" variant="outline" class="shrink-0">{{ t('emailRules.disabled') }}</Badge>
        </div>
        <p class="flex min-w-0 gap-3 truncate text-meta text-muted-foreground">
          <span class="truncate">{{ r.senderPattern }}</span>
          <span>{{ t(`emailRules.kind.${r.kind}`) }}</span>
          <span :class="accountName(r.accountId) ? '' : 'text-warning'">{{ accountName(r.accountId) ?? t('emailRules.noAccount') }}</span>
          <span :class="cn('truncate', target(r).name ? '' : 'text-warning')">{{ target(r).name ?? target(r).missing }}</span>
        </p>
      </div>
      <Switch
        :model-value="r.enabled" :disabled="busy === r.id" :aria-label="t('emailRules.toggle', { name: r.name })"
        @update:model-value="$emit('toggle', r, $event)"
      />
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button variant="ghost" size="icon" :class="cn('size-8', reveal)" :aria-label="t('emailRules.menu', { name: r.name })" data-testid="rule-menu">
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem @select="$emit('edit', r)">{{ t('actions.edit') }}</DropdownMenuItem>
          <DropdownMenuItem @select="$emit('duplicate', r)">{{ t('emailRules.duplicate') }}</DropdownMenuItem>
          <DropdownMenuItem variant="destructive" @select="$emit('delete', r)">{{ t('emailRules.delete') }}</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  </ul>
</template>
