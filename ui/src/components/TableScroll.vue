<script setup lang="ts">
import { useEventListener, useResizeObserver } from '@vueuse/core'
import { onMounted, ref, useTemplateRef } from 'vue'

// Wadah tabel data yang boleh di-scroll horizontal sendiri saat sempit (ukuran teks besar, jendela kecil);
// halaman tetap tidak meluap. Tabel data dikecualikan dari reflow (WCAG 1.4.10), docs/design/design.md §12.
// Bisa difokus (lalu digeser dengan panah) hanya ketika memang bisa di-scroll.
defineProps<{ label: string }>()

const region = useTemplateRef<HTMLElement>('region')
const scrollable = ref(false)
function measure() {
  const el = region.value
  scrollable.value = !!el && el.scrollWidth > el.clientWidth + 1
}
onMounted(measure)
useResizeObserver(region, measure)
useEventListener(window, 'resize', measure)
</script>

<template>
  <div
    ref="region" role="region" :aria-label="label" :tabindex="scrollable ? 0 : undefined"
    class="-mx-2 overflow-x-auto rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-ring"
  >
    <div class="w-max min-w-full"><slot /></div>
  </div>
</template>
