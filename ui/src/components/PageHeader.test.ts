import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PageHeader from './PageHeader.vue'

describe('PageHeader', () => {
  it('merender ringkasan dan aksi tanpa judul (judul ada di SiteHeader)', () => {
    const w = mount(PageHeader, {
      slots: { default: 'Kekayaan bersih Rp 1', actions: '<button>Akun baru</button>' },
    })
    expect(w.find('h1').exists()).toBe(false)
    expect(w.find('[data-part="summary"]').text()).toBe('Kekayaan bersih Rp 1')
    expect(w.find('[data-part="actions"] button').text()).toBe('Akun baru')
  })

  it('tanpa slot tidak merender apa pun', () => {
    const w = mount(PageHeader)
    expect(w.find('[data-part]').exists()).toBe(false)
  })
})
