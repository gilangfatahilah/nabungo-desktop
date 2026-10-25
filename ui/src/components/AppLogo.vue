<script setup lang="ts">
// Wordmark Nabungo: koin yang masuk ke celah tabungan (docs/design/brand/). Tanda memakai
// --primary agar ikut tema; teks memakai font aplikasi, bukan gambar.
</script>

<template>
  <span class="flex items-center gap-2" data-testid="app-logo">
    <svg viewBox="0 0 400 332" class="h-[1.125rem] w-auto shrink-0 text-primary" fill="currentColor" aria-hidden="true">
      <path d="M90.8 265A155 155 0 1 1 309.2 265Z" />
      <rect y="285" width="400" height="47" rx="23.5" />
    </svg>
    <span class="text-heading font-bold tracking-tight lowercase">{{ $t('app.name') }}</span>
  </span>
</template>
