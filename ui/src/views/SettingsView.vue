<script setup lang="ts">
import PageHeader from '@/components/PageHeader.vue'
import { useI18n } from 'vue-i18n'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { LocaleDto, ThemeDto } from '@/ipc'
import { withErrorToast } from '@/lib/notify'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const { t } = useI18n()

const themes: ThemeDto[] = ['light', 'dark', 'system']
const locales: LocaleDto[] = ['id', 'en']
const days = Array.from({ length: 28 }, (_, i) => i + 1)

function onTheme(v: unknown) {
  if (typeof v === 'string' && (themes as string[]).includes(v)) void withErrorToast(() => settings.setTheme(v as ThemeDto))
}
function onLocale(v: unknown) {
  if (typeof v === 'string' && (locales as string[]).includes(v)) void withErrorToast(() => settings.setLocale(v as LocaleDto))
}
function onDay(v: unknown) {
  const day = Number(v)
  if (days.includes(day)) void withErrorToast(() => settings.setPeriodStartDay(day))
}
</script>

<template>
  <PageHeader :title="t('nav.settings')" />
  <section class="max-w-md">
    <div class="flex flex-col gap-5">
      <div class="flex flex-col gap-2">
        <label id="theme-label" class="text-sm font-medium">{{ t('settings.theme') }}</label>
        <Select :model-value="settings.theme" @update:model-value="onTheme">
          <SelectTrigger aria-labelledby="theme-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="v in themes" :key="v" :value="v">{{ t(`settings.themes.${v}`) }}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="flex flex-col gap-2">
        <label id="locale-label" class="text-sm font-medium">{{ t('settings.language') }}</label>
        <Select :model-value="settings.locale" @update:model-value="onLocale">
          <SelectTrigger aria-labelledby="locale-label" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="v in locales" :key="v" :value="v">{{ t(`settings.locales.${v}`) }}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="flex flex-col gap-2">
        <label id="period-label" class="text-sm font-medium">{{ t('settings.periodStartDay') }}</label>
        <Select :model-value="String(settings.periodStartDay)" @update:model-value="onDay">
          <SelectTrigger aria-labelledby="period-label" aria-describedby="period-hint" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="d in days" :key="d" :value="String(d)">{{ d }}</SelectItem>
          </SelectContent>
        </Select>
        <p id="period-hint" class="text-xs text-muted-foreground">{{ t('settings.periodStartDayHint') }}</p>
      </div>
    </div>
  </section>
</template>
