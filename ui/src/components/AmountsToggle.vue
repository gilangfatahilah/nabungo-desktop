<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Eye, EyeOff } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { useUiStore } from '@/stores/ui'

// Toggle global mode sembunyikan nominal (docs/design/design.md §7.10): SiteHeader dan hero Dasbor.
// Provider sendiri agar bisa dirender di luar SidebarProvider (test kartu/layar).
const props = withDefaults(defineProps<{ size?: 'icon' | 'icon-sm' }>(), { size: 'icon' })
const ui = useUiStore()
const { t } = useI18n()
const label = computed(() => t(ui.amountsHidden ? 'privacy.show' : 'privacy.hide'))
</script>

<template>
  <TooltipProvider>
    <Tooltip>
      <TooltipTrigger as-child>
        <Button
          variant="ghost" :size="props.size" data-testid="toggle-amounts"
          :aria-label="label" :aria-pressed="ui.amountsHidden"
          @click="ui.toggleAmountsHidden()"
        >
          <EyeOff v-if="ui.amountsHidden" aria-hidden="true" />
          <Eye v-else aria-hidden="true" />
        </Button>
      </TooltipTrigger>
      <TooltipContent class="flex items-center gap-2">
        {{ label }}
        <KbdGroup><Kbd>Ctrl</Kbd><Kbd>Shift</Kbd><Kbd>H</Kbd></KbdGroup>
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>
