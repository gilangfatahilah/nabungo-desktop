import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PageHeader from './PageHeader.vue'

describe('PageHeader', () => {
  it('merender judul sebagai h1, ringkasan, dan aksi', () => {
    const w = mount(PageHeader, {
      props: { title: 'Akun' },
      slots: { default: 'Kekayaan bersih Rp 1', actions: '<button>Akun baru</button>' },
    })
    expect(w.find('h1').text()).toBe('Akun')
    expect(w.find('[data-part="summary"]').text()).toBe('Kekayaan bersih Rp 1')
    expect(w.find('[data-part="actions"] button').text()).toBe('Akun baru')
  })

  it('tanpa slot tidak merender wadah kosong', () => {
    const w = mount(PageHeader, { props: { title: 'Dasbor' } })
    expect(w.find('[data-part="summary"]').exists()).toBe(false)
    expect(w.find('[data-part="actions"]').exists()).toBe(false)
  })
})
