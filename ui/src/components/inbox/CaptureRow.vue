<script setup lang="ts">
import { EllipsisVertical, Mail } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import type { CaptureDto } from '@/ipc'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'

// Satu draft email capture (docs/design/design.md §8.10). Referensi kosong/arsip ditulis sebagai ajakan memilih.
const props = defineProps<{ capture: CaptureDto; ready: boolean; busy: boolean }>()
defineEmits<{ confirm: []; review: []; ignore: []; duplicates: [] }>()

const { t } = useI18n()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const reveal = 'opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 data-[state=open]:opacity-100'

const accountName = (id: string | null) => accounts.overview.accounts.find((a) => a.id === id && !a.archived)?.name ?? null
const categoryName = (id: string | null) => {
  const c = categories.list.find((x) => x.id === id && !x.archived)
  return c && c.kind === props.capture.kind ? c.name : null
}

const title = computed(() => props.capture.payee || props.capture.ruleName)
const account = computed(() => accountName(props.capture.accountId))
const target = computed(() =>
  props.capture.kind === 'transfer'
    ? { name: accountName(props.capture.toAccountId), missing: t('inbox.pickAccount') }
    : { name: categoryName(props.capture.categoryId), missing: t('inbox.pickCategory') },
)
const amount = computed(() => (props.capture.kind === 'expense' ? -props.capture.amount : props.capture.amount))
const tone = computed(() => (props.capture.kind === 'transfer' ? 'neutral' : props.capture.kind))
const hasDuplicates = computed(() => props.capture.duplicates.length > 0)
</script>

<template>
  <li
    data-testid="capture-row" :data-capture-id="capture.id"
    class="group/row flex min-h-12 items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-muted"
  >
    <span class="flex size-7 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground" aria-hidden="true">
      <Mail class="size-4" />
    </span>
    <div class="min-w-0 flex-1">
      <div class="flex min-w-0 items-center gap-2">
        <span data-part="title" class="truncate font-medium">{{ title }}</span>
        <button
          v-if="hasDuplicates" type="button" data-testid="capture-duplicates"
          class="shrink-0 rounded-md border border-warning/50 px-1.5 text-meta text-warning hover:bg-warning/10 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          @click="$emit('duplicates')"
        >
          {{ t('inbox.duplicate') }}
        </button>
      </div>
      <p class="flex min-w-0 gap-3 truncate text-meta text-muted-foreground">
        <span class="truncate">{{ capture.ruleName }}</span>
        <span :class="account ? '' : 'text-warning'">{{ account ?? t('inbox.pickAccount') }}</span>
        <span :class="cn('truncate', target.name ? '' : 'text-warning')">{{ target.name ?? target.missing }}</span>
      </p>
    </div>
    <MoneyText :amount="amount" :kind="tone" class="w-32 shrink-0 text-right font-medium" />
    <div class="flex shrink-0 items-center gap-1">
      <Button v-if="ready" size="sm" data-testid="capture-record" :disabled="busy" @click="$emit('confirm')">{{ t('inbox.record') }}</Button>
      <Button size="sm" variant="outline" data-testid="capture-review" :disabled="busy" @click="$emit('review')">{{ t('inbox.review') }}</Button>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button variant="ghost" size="icon" :class="cn('size-8', reveal)" :aria-label="t('actions.more')" data-testid="capture-menu" :disabled="busy">
            <EllipsisVertical />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem v-if="hasDuplicates" data-testid="capture-link" @select="$emit('duplicates')">{{ t('inbox.linkMenu') }}</DropdownMenuItem>
          <DropdownMenuItem data-testid="capture-ignore" @select="$emit('ignore')">{{ t('inbox.ignore') }}</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  </li>
</template>
