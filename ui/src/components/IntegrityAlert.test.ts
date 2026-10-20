import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import IntegrityAlert from '@/components/IntegrityAlert.vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { routes } from '@/router'

async function mountAlert(integrity: ipc.IntegrityStatusDto) {
  const report = vi.spyOn(ipc, 'startupReport').mockResolvedValue({ uncleanShutdown: integrity !== 'skipped', integrity })
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push('/')
  const w = mount(IntegrityAlert, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  return { w, router, report }
}

const dialog = () => document.body.querySelector('[role="alertdialog"]')

describe('IntegrityAlert', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('failed menampilkan dialog dan tombol membuka pengaturan', async () => {
    const { w, router, report } = await mountAlert('failed')
    expect(report).toHaveBeenCalledTimes(1)
    expect(dialog()?.textContent).toContain(id.integrity.title)
    const open = [...dialog()!.querySelectorAll('button')].find((b) => b.textContent?.trim() === id.integrity.openBackups)!
    open.click()
    await flushPromises()
    await vi.waitFor(() => expect(router.currentRoute.value.name).toBe('settings'))
    expect(dialog()).toBeNull()
    w.unmount()
  })

  it.each(['ok', 'skipped'] as const)('%s tidak menampilkan apa pun', async (integrity) => {
    const { w } = await mountAlert(integrity)
    expect(dialog()).toBeNull()
    w.unmount()
  })
})
