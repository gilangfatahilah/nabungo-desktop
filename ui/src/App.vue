<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
import 'vue-sonner/style.css'
</script>

<template>
  <div class="flex h-screen bg-background text-foreground">
    <aside class="w-60 shrink-0 border-r p-4">
      <p class="text-lg font-semibold">{{ $t('app.name') }}</p>
      <nav class="mt-6 flex flex-col gap-1 text-sm">
        <RouterLink to="/" class="rounded-md px-2 py-1.5 hover:bg-accent">{{ $t('nav.dashboard') }}</RouterLink>
      </nav>
    </aside>
    <main class="flex-1 overflow-auto p-6">
      <RouterView />
    </main>
    <Toaster />
  </div>
</template>
