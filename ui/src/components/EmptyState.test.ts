import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import EmptyState from './EmptyState.vue'

describe('EmptyState', () => {
  it('merender judul, penjelasan, dan aksi', () => {
    const w = mount(EmptyState, {
      props: { title: 'Belum ada transaksi bulan ini.', description: 'Catat pengeluaran pertama.' },
      slots: { default: '<button>Catat transaksi</button>' },
    })
    expect(w.find('[data-part="title"]').text()).toBe('Belum ada transaksi bulan ini.')
    expect(w.find('[data-part="description"]').text()).toBe('Catat pengeluaran pertama.')
    expect(w.find('button').text()).toBe('Catat transaksi')
  })
})
