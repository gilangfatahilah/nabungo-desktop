<script setup lang="ts">
import BackupSection from '@/components/settings/BackupSection.vue'
import EmailSection from '@/components/settings/EmailSection.vue'
import ExportSection from '@/components/settings/ExportSection.vue'
import SecuritySection from '@/components/settings/SecuritySection.vue'
import UpdateSection from '@/components/settings/UpdateSection.vue'
import { Separator } from '@/components/ui/separator'
import { nextTick, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { LocaleDto, ThemeDto } from '@/ipc'
import { FONT_SCALES, isFontScale } from '@/lib/fontScale'
import { withErrorToast } from '@/lib/notify'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const { t } = useI18n()
const route = useRoute()

// `?section=update` (dari toast update tersedia): langsung ke section-nya.
onMounted(async () => {
  if (typeof route.query.section !== 'string') return
  await nextTick()
  document.getElementById(`settings-${route.query.section}`)?.scrollIntoView({ block: 'start' })
})

const themes: ThemeDto[] = ['light', 'dark', 'system']
const locales: LocaleDto[] = ['id', 'en']
const days = Array.from({ length: 28 }, (_, i) => i + 1)

function onTheme(v: unknown) {
  if (typeof v === 'string' && (themes as string[]).includes(v)) void withErrorToast(() => settings.setTheme(v as ThemeDto))
}
function onFontScale(v: unknown) {
  const s = Number(v)
  if (isFontScale(s)) void withErrorToast(() => settings.setFontScale(s))
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
  <section aria-labelledby="settings-general" class="max-w-md">
    <h2 id="settings-general" class="mb-4 text-heading">{{ t('settings.sections.general') }}</h2>
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
        <label id="font-scale-label" class="text-sm font-medium">{{ t('settings.fontScale') }}</label>
        <Select :model-value="String(settings.fontScale)" @update:model-value="onFontScale">
          <SelectTrigger aria-labelledby="font-scale-label" aria-describedby="font-scale-hint" class="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem v-for="s in FONT_SCALES" :key="s" :value="String(s)">{{ s }}%</SelectItem>
          </SelectContent>
        </Select>
        <p id="font-scale-hint" class="text-xs text-muted-foreground">{{ t('settings.fontScaleHint') }}</p>
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
  <Separator class="my-8" />
  <section aria-labelledby="settings-security">
    <h2 id="settings-security" class="mb-4 text-heading">{{ t('settings.sections.security') }}</h2>
    <SecuritySection />
  </section>
  <Separator class="my-8" />
  <section aria-labelledby="settings-email">
    <h2 id="settings-email" class="mb-4 text-heading">{{ t('settings.sections.email') }}</h2>
    <EmailSection />
  </section>
  <Separator class="my-8" />
  <section aria-labelledby="settings-backup">
    <h2 id="settings-backup" class="mb-4 text-heading">{{ t('settings.sections.backup') }}</h2>
    <BackupSection />
  </section>
  <Separator class="my-8" />
  <section aria-labelledby="settings-export" class="max-w-md">
    <h2 id="settings-export" class="mb-4 text-heading">{{ t('settings.sections.export') }}</h2>
    <ExportSection />
  </section>
  <Separator class="my-8" />
  <section aria-labelledby="settings-update">
    <h2 id="settings-update" class="mb-4 text-heading">{{ t('settings.sections.update') }}</h2>
    <UpdateSection />
  </section>
</template>
