<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { LocaleDto, ThemeDto } from '@/ipc'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const { t } = useI18n()

const themes: ThemeDto[] = ['light', 'dark', 'system']
const locales: LocaleDto[] = ['id', 'en']

function onTheme(v: unknown) {
  if (typeof v === 'string' && (themes as string[]).includes(v)) void settings.setTheme(v as ThemeDto)
}
function onLocale(v: unknown) {
  if (typeof v === 'string' && (locales as string[]).includes(v)) void settings.setLocale(v as LocaleDto)
}
</script>

<template>
  <section class="max-w-md">
    <h1 class="text-2xl font-semibold">{{ t('nav.settings') }}</h1>
    <div class="mt-6 flex flex-col gap-5">
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
    </div>
  </section>
</template>
