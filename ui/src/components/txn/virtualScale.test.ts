import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick, type Component } from 'vue'
import TxnList from '@/components/txn/TxnList.vue'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import { rootFontScale } from '@/lib/fontScale'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'

// Estimasi tinggi baris virtual harus mengikuti ukuran teks (rem), bukan px tetap.
vi.mock('@/lib/fontScale', async (orig) => ({ ...(await orig<typeof import('@/lib/fontScale')>()), rootFontScale: vi.fn(() => 1) }))

beforeAll(async () => {
  const bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-10-01', contact: null },
  })
  const cat = (await ipc.listCategories()).find((c) => c.name === 'Kopi & jajan')!.id
  await ipc.createTxn({ kind: 'expense', date: '2026-10-04', accountId: bca, toAccountId: null,
    amount: 25_000, splits: [{ categoryId: cat, amount: 25_000, memo: null }], fee: null,
    payee: null, note: null, tags: [] })
})

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

// Div/tabel pengisi virtualizer: tingginya = jumlah estimasi semua baris.
function totalHeight(): number {
  const sized = [...document.body.querySelectorAll<HTMLElement>('[style*="height"]')]
  return sized.reduce((sum, el) => sum + (parseFloat(el.style.height) || 0), 0)
}

async function heightAt(component: Component, scale: number): Promise<number> {
  document.body.innerHTML = ''
  setActivePinia(createPinia())
  vi.mocked(rootFontScale).mockReturnValue(scale)
  await useTransactionsStore().reload()
  const w = mount(component, { global: { plugins: [i18n] }, attachTo: document.body })
  await settle()
  const h = totalHeight()
  w.unmount()
  return h
}

describe('virtualizer mengikuti ukuran teks', () => {
  beforeEach(() => vi.mocked(rootFontScale).mockReturnValue(1))

  // TxnTable memakai mekanisme yang sama, tetapi di jsdom tidak merender baris pengisi sama sekali.
  it('TxnList: estimasi baris dikali skala', async () => {
    const base = await heightAt(TxnList, 1)
    expect(base).toBeGreaterThan(0)
    expect(await heightAt(TxnList, 1.25)).toBeCloseTo(base * 1.25)
  })

  it('TxnList mengukur ulang saat ukuran teks berubah', async () => {
    const base = await heightAt(TxnList, 1)
    await useTransactionsStore().reload()
    mount(TxnList, { global: { plugins: [i18n] }, attachTo: document.body })
    await settle()
    vi.mocked(rootFontScale).mockReturnValue(1.5)
    await useSettingsStore().setFontScale(150)
    await settle()
    expect(totalHeight()).toBeCloseTo(base * 1.5)
  })
})
