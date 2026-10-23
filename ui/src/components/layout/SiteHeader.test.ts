import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { defineComponent, h } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import SiteHeader from '@/components/layout/SiteHeader.vue'
import { SidebarProvider } from '@/components/ui/sidebar'
import { i18n } from '@/i18n'
import { routes } from '@/router'
import { useUiStore } from '@/stores/ui'

beforeEach(() => setActivePinia(createPinia()))

async function mountHeader(path: string) {
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push(path)
  const Host = defineComponent(() => () => h(SidebarProvider, null, () => h(SiteHeader)))
  const w = mount(Host, { global: { plugins: [i18n, router] } })
  await flushPromises()
  return w
}

describe('SiteHeader', () => {
  it('judul halaman (h1) diambil dari route', async () => {
    expect((await mountHeader('/budget')).find('h1').text()).toBe('Budget')
    expect((await mountHeader('/labels')).find('h1').text()).toBe('Tag & Pihak')
  })

  it('tombol Cari membuka command palette', async () => {
    const w = await mountHeader('/')
    await w.find('[data-testid="open-palette"]').trigger('click')
    expect(useUiStore().paletteOpen).toBe(true)
  })

  it('toggle sidebar berlabel terjemahan', async () => {
    const w = await mountHeader('/')
    expect(w.find('[data-sidebar="trigger"]').text()).toBe('Buka atau ciutkan sidebar')
  })
})
