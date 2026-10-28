<script setup lang="ts">
import { ChevronRight } from '@lucide/vue'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import EmptyState from '@/components/EmptyState.vue'
import { Button } from '@/components/ui/button'
import type { UnparsedEmailDto } from '@/ipc'
import { formatTimestamp } from '@/lib/date'
import { cn } from '@/lib/utils'
import { useSettingsStore } from '@/stores/settings'

// Email yang tidak cocok dengan rule mana pun. `Buat rule` hanya untuk email yang pengirimnya terbaca.
defineProps<{ items: UnparsedEmailDto[] }>()
defineEmits<{ createRule: [messageId: string] }>()

const { t } = useI18n()
const settings = useSettingsStore()
const open = ref(new Set<string>())

function toggle(id: string) {
  const next = new Set(open.value)
  if (!next.delete(id)) next.add(id)
  open.value = next
}
</script>

<template>
  <EmptyState v-if="!items.length" :title="t('inbox.unparsedEmpty')" />
  <ul v-else class="-mx-2 flex flex-col">
    <li v-for="u in items" :key="u.messageId" data-testid="unparsed-row" :data-message-id="u.messageId" class="rounded-lg">
      <div class="flex items-center gap-2 pr-2">
      <button
        type="button" :aria-expanded="open.has(u.messageId)"
        class="flex min-h-12 w-full items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        @click="toggle(u.messageId)"
      >
        <ChevronRight :class="cn('size-4 shrink-0 text-muted-foreground transition-transform', open.has(u.messageId) && 'rotate-90')" aria-hidden="true" />
        <span class="min-w-0 flex-1">
          <span class="block truncate font-medium">{{ u.subject || t('inbox.noSubject') }}</span>
          <span class="block truncate text-meta text-muted-foreground">{{ u.sender }}</span>
        </span>
        <span v-if="u.receivedAt" class="num shrink-0 text-meta text-muted-foreground">{{ formatTimestamp(u.receivedAt, settings.locale) }}</span>
      </button>
      <Button
        v-if="u.sender" variant="outline" size="sm" class="shrink-0" data-testid="unparsed-create-rule"
        @click="$emit('createRule', u.messageId)"
      >
        {{ t('emailRules.createFromEmail') }}
      </Button>
      </div>
      <p v-if="open.has(u.messageId)" class="ml-9 max-w-prose px-2 pb-3 whitespace-pre-line text-muted-foreground">{{ u.snippet }}</p>
    </li>
  </ul>
</template>
