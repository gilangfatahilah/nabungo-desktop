<script setup lang="ts">
// Wordmark Nabungo: huruf N berfaset dengan koin yang jatuh masuk (docs/design/brand/nabungo-mark.svg).
// Tanda memakai warna brand tetap (gradien adalah identitasnya, tidak ikut tema); teks memakai
// font aplikasi, bukan gambar.
import { useId } from 'vue'

// ID gradien unik per instance agar dua logo di satu dokumen tidak saling menimpa.
const id = useId()
</script>

<template>
  <span class="flex items-center gap-2" data-testid="app-logo">
    <svg viewBox="0 0 317 433" class="h-5 w-auto shrink-0" aria-hidden="true">
      <defs>
        <linearGradient :id="`${id}-l`" x1="0" y1="128" x2="0" y2="421" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#417A35" />
          <stop offset="1" stop-color="#145541" />
        </linearGradient>
        <linearGradient :id="`${id}-d`" x1="80" y1="128" x2="238" y2="433" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#56A046" />
          <stop offset="1" stop-color="#38864F" />
        </linearGradient>
        <linearGradient :id="`${id}-r`" x1="0" y1="105" x2="0" y2="433" gradientUnits="userSpaceOnUse">
          <stop offset="0" stop-color="#8AD23E" />
          <stop offset="1" stop-color="#4AA356" />
        </linearGradient>
      </defs>
      <path :fill="`url(#${id}-l)`" d="M0 173 80 128v293L0 388Z" />
      <path :fill="`url(#${id}-d)`" d="M80 128 238 330v103L80 230Z" />
      <path :fill="`url(#${id}-r)`" d="M238 148 317 105v283l-79 45Z" />
      <path fill="#FFFFFF" fill-opacity=".28" d="M238 148 317 105v45l-79 46Z" />
      <circle cx="278" cy="31" r="31" fill="#A3E635" />
    </svg>
    <span class="text-heading font-bold tracking-tight lowercase">{{ $t('app.name') }}</span>
  </span>
</template>
