import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { rootFontScale } from '@/lib/fontScale'
import LineChart from './LineChart.vue'

// Kartu selebar 750px; teks sumbu harus ikut ukuran teks, jadi koordinat viewBox = lebar / skala.
vi.mock('@vueuse/core', async (orig) => ({ ...(await orig<typeof import('@vueuse/core')>()), useElementSize: () => ({ width: ref(750), height: ref(200) }) }))
vi.mock('@/lib/fontScale', async (orig) => ({ ...(await orig<typeof import('@/lib/fontScale')>()), rootFontScale: vi.fn(() => 1) }))

const props = { labels: ['Jan', 'Feb'], series: [{ key: 'a', label: 'A', color: 'red', values: [1, 2] }], label: 'chart', format: String }
const viewBox = () => mount(LineChart, { props }).find('svg').attributes('viewBox')

describe('LineChart mengikuti ukuran teks', () => {
  it('skala 1: viewBox selebar elemen', () => {
    expect(viewBox()).toBe('0 0 750 200')
  })

  it('skala 1.25: viewBox dipersempit sehingga teks 11 unit tampil 13.75px', () => {
    vi.mocked(rootFontScale).mockReturnValue(1.25)
    expect(viewBox()).toBe('0 0 600 200')
  })
})
