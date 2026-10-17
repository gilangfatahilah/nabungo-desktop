<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { colorClass, COLORS } from '@/lib/colors'
import { ICONS, iconFor } from '@/lib/icons'
import { cn } from '@/lib/utils'

const icon = defineModel<string | null>('icon', { required: true })
const color = defineModel<string | null>('color', { required: true })
const props = defineProps<{ id?: string; fallbackIcon?: string | null }>()

const names = Object.keys(ICONS)
</script>

<template>
  <Popover>
    <PopoverTrigger as-child>
      <Button :id="props.id" type="button" variant="outline" class="w-fit gap-2">
        <span :class="cn('flex size-6 items-center justify-center rounded-md', colorClass(color))">
          <component :is="iconFor(icon ?? props.fallbackIcon ?? null)" class="size-4" />
        </span>
        {{ $t('accounts.appearance') }}
      </Button>
    </PopoverTrigger>
    <PopoverContent class="w-72">
      <div class="grid grid-cols-7 gap-1" role="radiogroup" :aria-label="$t('accounts.appearance')">
        <Button
          v-for="n in names"
          :key="n"
          type="button"
          size="icon-sm"
          :variant="icon === n ? 'secondary' : 'ghost'"
          role="radio"
          :aria-checked="icon === n"
          :aria-label="n"
          @click="icon = icon === n ? null : n"
        >
          <component :is="ICONS[n]" />
        </Button>
      </div>
      <div class="mt-3 flex flex-wrap gap-1.5" role="radiogroup" :aria-label="$t('actions.noColor')">
        <button
          type="button"
          role="radio"
          :aria-checked="color === null"
          :aria-label="$t('actions.noColor')"
          :class="cn('size-6 rounded-full border', colorClass(null), color === null && 'ring-2 ring-ring')"
          @click="color = null"
        />
        <button
          v-for="c in COLORS"
          :key="c"
          type="button"
          role="radio"
          :aria-checked="color === c"
          :aria-label="c"
          :class="cn('size-6 rounded-full', colorClass(c), color === c && 'ring-2 ring-ring')"
          @click="color = c"
        />
      </div>
    </PopoverContent>
  </Popover>
</template>
