<script setup lang="ts">
import { Toaster } from '@/components/ui/sonner'
import 'vue-sonner/style.css'
import { onMounted } from 'vue'
import AppSidebar from '@/components/AppSidebar.vue'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'
import { useSettingsStore } from '@/stores/settings'

const settings = useSettingsStore()
onMounted(() => {
  void settings.load()
})
</script>

<template>
  <SidebarProvider>
    <AppSidebar />
    <SidebarInset class="h-screen overflow-auto">
      <main class="flex-1 p-6">
        <RouterView />
      </main>
    </SidebarInset>
    <Toaster />
  </SidebarProvider>
</template>
