<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import { Loader2Icon } from '@lucide/vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import { Switch } from '@/components/ui/switch'
import * as ipc from '@/ipc'
import type { UpdateStatusDto } from '@/ipc'
import { formatBytes } from '@/lib/bytes'
import { formatTimestamp } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { withErrorToast } from '@/lib/notify'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
const { t } = useI18n()
const tr = t as unknown as Translate

const status = ref<UpdateStatusDto | null>(null)
const info = computed(() => status.value?.lastCheck ?? null)
const kind = computed(() => status.value?.installKind ?? null)
const checking = ref(false)
/** Kemajuan unduhan yang sedang berjalan; `null` = tidak mengunduh. */
const progress = ref<{ downloaded: number; total: number } | null>(null)
const installing = ref(false)
const confirmOpen = ref(false)

const isWindows = computed(() => kind.value === 'nsis' || kind.value === 'msi')
const percent = computed(() =>
  progress.value && progress.value.total > 0 ? Math.round((progress.value.downloaded / progress.value.total) * 100) : 0,
)
const published = computed(() =>
  info.value?.publishedAt ? t('update.publishedAt', { when: formatTimestamp(info.value.publishedAt, settings.locale) }) : null,
)

async function refresh() {
  status.value = await ipc.updateStatus()
}

let off: (() => void) | null = null
onMounted(async () => {
  await withErrorToast(refresh)
  off = await ipc.onUpdate({ progress: (p) => (progress.value = p) })
})
onUnmounted(() => off?.())

async function setAutoCheck(enabled: boolean) {
  await withErrorToast(async () => {
    await ipc.setUpdateAutoCheck(enabled)
    await refresh()
  })
}

async function check() {
  checking.value = true
  await withErrorToast(async () => {
    await ipc.checkUpdate()
    await refresh()
  })
  checking.value = false
}

async function download() {
  progress.value = { downloaded: 0, total: info.value?.installer?.size ?? 0 }
  try {
    await ipc.downloadUpdate()
  } catch (e) {
    progress.value = null
    // Batal atas permintaan user: tidak perlu toast galat.
    if (!(e instanceof ipc.IpcFailure && e.error.code === 'update_cancelled')) toast.error(errorMessage(e, tr))
    return
  }
  progress.value = null
  await withErrorToast(refresh)
  // Windows menutup aplikasi: tanya dulu.
  if (isWindows.value) confirmOpen.value = true
  else await install()
}

async function install() {
  installing.value = true
  await withErrorToast(async () => {
    const outcome = await ipc.installUpdate()
    if (outcome === 'installer_opened') toast.success(t('update.installerOpened'))
    else if (outcome === 'revealed') toast.success(t('update.revealed'))
  })
  installing.value = false
}

function primaryAction() {
  if (!status.value?.downloaded) void download()
  else if (isWindows.value) confirmOpen.value = true
  else void install()
}

const primaryLabel = computed(() => {
  if (status.value?.downloaded) return kind.value === 'appimage' ? t('update.restart') : t('update.install')
  return kind.value === 'appimage' ? t('update.downloadRestart') : t('update.download')
})
</script>

<template>
  <div class="flex max-w-2xl flex-col gap-4">
    <p class="max-w-prose text-muted-foreground">{{ t('update.intro') }}</p>
    <p class="font-medium" data-testid="update-current">
      {{ t('update.current', { version: status?.current ?? '' }) }}
    </p>

    <div class="flex items-start gap-3">
      <Switch
        id="update-auto"
        :model-value="status?.autoCheck ?? false"
        :disabled="status === null"
        aria-describedby="update-auto-hint"
        class="mt-0.5"
        @update:model-value="(v: boolean) => setAutoCheck(v)"
      />
      <div class="flex flex-col gap-1">
        <label for="update-auto" class="text-sm font-medium">{{ t('update.autoCheck') }}</label>
        <p id="update-auto-hint" class="text-meta text-muted-foreground">{{ t('update.autoCheckHint') }}</p>
      </div>
    </div>

    <div>
      <Button variant="outline" :disabled="checking || progress !== null" @click="check">
        <Loader2Icon v-if="checking" class="size-4 animate-spin" aria-hidden="true" />
        {{ t('update.check') }}
      </Button>
    </div>

    <p v-if="info && !info.available" class="text-muted-foreground" role="status" data-testid="update-uptodate">
      {{ t('update.upToDate', { version: info.current }) }}
    </p>

    <div v-if="info?.available" class="flex flex-col gap-3 rounded-lg border p-4" data-testid="update-available">
      <div class="flex flex-col gap-1">
        <h3 class="font-semibold">{{ t('update.available', { version: info.latest }) }}</h3>
        <p v-if="published" class="text-meta text-muted-foreground">{{ published }}</p>
      </div>
      <div class="flex flex-col gap-1">
        <h4 class="text-meta font-medium text-muted-foreground">{{ t('update.notes') }}</h4>
        <!-- Teks polos: catatan rilis tidak dirender sebagai markdown/HTML. -->
        <p class="max-h-48 overflow-auto whitespace-pre-wrap text-sm" tabindex="0">
          {{ info.notes.trim() || t('update.noNotes') }}
        </p>
      </div>

      <p v-if="kind === null" class="text-meta text-muted-foreground">{{ t('update.manualOnly') }}</p>
      <p v-else-if="!info.installer" class="text-meta text-muted-foreground">{{ t('update.noInstaller') }}</p>

      <div v-if="progress" class="flex flex-col gap-2" aria-live="polite">
        <Progress :model-value="percent" :aria-label="t('settings.sections.update')" />
        <div class="flex items-center justify-between gap-3">
          <span class="text-meta text-muted-foreground">
            {{
              t('update.downloading', {
                done: formatBytes(progress.downloaded, settings.locale),
                total: formatBytes(progress.total, settings.locale),
              })
            }}
          </span>
          <Button variant="outline" size="sm" @click="ipc.cancelUpdateDownload()">{{ t('actions.cancel') }}</Button>
        </div>
      </div>
      <p v-else-if="status?.downloaded" class="text-meta text-muted-foreground">{{ t('update.downloaded') }}</p>

      <div class="flex flex-wrap gap-2">
        <Button v-if="kind !== null && info.installer && !progress" :disabled="installing" @click="primaryAction">
          <Loader2Icon v-if="installing" class="size-4 animate-spin" aria-hidden="true" />
          {{ primaryLabel }}
          <span v-if="!status?.downloaded" class="text-primary-foreground/80">
            ({{ formatBytes(info.installer.size, settings.locale) }})
          </span>
        </Button>
        <Button variant="outline" @click="withErrorToast(ipc.openReleasePage)">{{ t('update.openReleasePage') }}</Button>
      </div>
    </div>

    <ConfirmDialog
      v-model:open="confirmOpen"
      :title="t('update.confirmTitle')"
      :description="t('update.confirmBody')"
      :confirm-label="t('update.confirm')"
      @confirm="install"
    />
  </div>
</template>
