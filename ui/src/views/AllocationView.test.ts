import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { useTransactionsStore } from '@/stores/transactions'
import { emptyForm } from '@/lib/txnForm'
import AllocationView from './AllocationView.vue'

const $ = <T extends Element = HTMLElement>(sel: string) => document.body.querySelector(sel) as T
const text = () => document.body.textContent ?? ''
let bca = ''

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function view() {
  mount(AllocationView, { global: { plugins: [i18n] }, attachTo: document.body })
  await settle()
}

// Input shadcn memakai useVModel passive: model ikut ter-update di flush berikutnya.
async function type(sel: string, value: string) {
  const el = $<HTMLInputElement | HTMLSelectElement>(sel)
  el.value = value
  el.dispatchEvent(new Event(el.tagName === 'SELECT' ? 'change' : 'input', { bubbles: true }))
  await nextTick()
}

async function submit(testid: string) {
  $(`[data-testid="${testid}"]`).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
  await settle()
}

const bucketId = async (name: string) => (await ipc.allocationOverview()).buckets.find((b) => b.name === name)!.id

beforeAll(async () => {
  bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-09-01', contact: null },
  })
})

// State browser-ledger dipakai bersama: test berjalan berurutan dan saling membangun.
describe('AllocationView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('empty state lalu membuat bucket', async () => {
    await view()
    expect(text()).toContain(id.allocation.empty)
    $('[data-testid="bucket-new"]').click()
    await settle()
    await type('#bucket-name', 'Aset')
    await type('#bucket-target', '100')
    await submit('bucket-form')
    expect(text()).not.toContain(id.allocation.empty)
    const row = $(`[data-testid="bucket-row-${await bucketId('Aset')}"]`)
    expect(row.textContent).toContain('Aset')
    expect(row.textContent).toContain('100%')
    expect($('[data-testid="allocation-warnings"]')).toBeNull()
  })

  it('peringatan muncul saat target root bukan 100%', async () => {
    const aset = await bucketId('Aset')
    await ipc.updateBucket(aset, { name: 'Aset', parentId: null, targetBp: 6000, targetAmount: null, color: null })
    await ipc.createBucket({ name: 'Liquid', parentId: null, targetBp: 3000, targetAmount: null, color: null })
    await view()
    expect($('[data-testid="allocation-warnings"]').textContent).toContain('Target persen tingkat utama berjumlah 90%, bukan 100%')
  })

  it('memetakan akun memperbarui porsi aktual dan menghapus dari belum dialokasikan', async () => {
    await view()
    expect($(`[data-testid="mapping-${bca}"]`).textContent).toContain(id.allocation.unmapped)
    expect($('[data-testid="bucket-unallocated"]').textContent).toContain('100%')
    $(`[data-testid="map-account-${bca}"]`).click()
    await settle()
    await type('[data-testid="share-bucket-0"]', await bucketId('Aset'))
    await type('[data-testid="share-percent-0"]', '90')
    $('[data-testid="share-add"]').click()
    await settle()
    await type('[data-testid="share-bucket-1"]', await bucketId('Liquid'))
    await type('[data-testid="share-percent-1"]', '10')
    await settle()
    expect(text()).toContain('0% belum dialokasikan')
    await submit('allocation-form')
    expect($(`[data-testid="mapping-${bca}"]`).textContent).toContain('Aset 90% · Liquid 10%')
    expect($(`[data-testid="bucket-row-${await bucketId('Aset')}"]`).textContent).toContain('90%')
    expect($('[data-testid="bucket-unallocated"]')).toBeNull()
  })

  it('child menampilkan target relatif terhadap parent', async () => {
    const aset = await bucketId('Aset')
    await ipc.createBucket({ name: 'Saham', parentId: aset, targetBp: 6000, targetAmount: null, color: null })
    await view()
    const row = $(`[data-testid="bucket-row-${await bucketId('Saham')}"]`)
    expect(row.textContent).toContain('60% dari Aset')
    expect($(`[data-testid="bucket-target-${await bucketId('Saham')}"]`).textContent).toBe('36%')
  })

  it('memuat ulang setelah transaksi disimpan dari form global', async () => {
    await view()
    const aset = () => $(`[data-testid="bucket-row-${asetId}"]`).textContent
    const asetId = await bucketId('Aset')
    expect(aset()).toContain('900.000')
    const gaji = (await ipc.listCategories()).find((c) => c.name === 'Gaji')!.id
    await useTransactionsStore().save(null, {
      ...emptyForm('income', { accountId: bca, feeCategoryId: null, today: '2026-10-05' }),
      amount: 500_000, categoryId: gaji,
    })
    await settle()
    expect($(`[data-testid="mapping-${bca}"]`).textContent).toContain('1.500.000')
    expect(aset()).toContain('1.350.000')
  })

  it('membuat bucket nominal lewat dialog', async () => {
    await view()
    $('[data-testid="bucket-new"]').click()
    await settle()
    await type('#bucket-name', 'Darurat')
    $('[data-testid="bucket-kind-amount"]').click()
    await settle()
    await type('#bucket-amount', '300000')
    await submit('bucket-form')
    const darurat = (await ipc.allocationOverview()).buckets.find((b) => b.name === 'Darurat')!
    expect(darurat).toMatchObject({ targetBp: 0, targetAmount: 300_000 })
    expect($(`[data-testid="bucket-target-amount-${darurat.id}"]`).textContent).toContain('300.000')
    // Gross 1.500.000: nominal 300.000 = 20%.
    expect($(`[data-testid="bucket-target-${darurat.id}"]`).textContent).toBe('20%')
  })

  it('peringatan saat target nominal melebihi total aset', async () => {
    await ipc.createBucket({ name: 'Rumah', parentId: null, targetBp: 0, targetAmount: 2_000_000, color: null })
    await view()
    expect($('[data-testid="allocation-warnings"]').textContent)
      .toContain('Target nominal tingkat utama 2.300.000 melebihi total aset 1.500.000')
  })
})
