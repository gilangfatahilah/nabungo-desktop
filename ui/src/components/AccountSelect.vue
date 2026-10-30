<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import IconSelect, { type IconOption } from '@/components/IconSelect.vue'
import type { AccountDto } from '@/ipc'
import { KIND_ICONS } from '@/lib/accounts'

// Pemilih akun dengan kotak cari + ikon akun. Atribut jatuh ke tombol pemicu (lihat IconSelect).
// `noneLabel` menambah opsi "kosong" (nilai null) di atas daftar.
const model = defineModel<string | null>({ required: true })
const props = defineProps<{ accounts: AccountDto[]; noneLabel?: string; disabledId?: string | null }>()
defineOptions({ inheritAttrs: false })

const { t } = useI18n()
const options = computed<IconOption[]>(() => props.accounts.map((a) => ({
  value: a.id, label: a.name, icon: a.icon ?? KIND_ICONS[a.kind], color: a.color, disabled: a.id === props.disabledId,
})))
</script>

<template>
  <IconSelect
    v-bind="$attrs" v-model="model" :options="options" :none-label="noneLabel"
    :search-placeholder="t('accounts.search')" :empty-text="t('accounts.searchEmpty')"
  />
</template>
