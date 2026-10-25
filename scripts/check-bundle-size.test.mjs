import assert from 'node:assert/strict'
import { test } from 'node:test'
import { checkSizes, LIMIT } from './check-bundle-size.mjs'

test('batas 15 000 000 byte per installer', () => {
  assert.equal(LIMIT, 15_000_000)
  assert.equal(checkSizes([{ name: 'Nabungo_0.1.0_amd64.deb', bytes: 14_999_999 }]).ok, true)
  const r = checkSizes([{ name: 'Nabungo_0.1.0_x64_en-US.msi', bytes: 15_000_000 }])
  assert.equal(r.ok, false)
  assert.deepEqual(r.rows, [{ name: 'Nabungo_0.1.0_x64_en-US.msi', bytes: 15_000_000, limit: LIMIT, ok: false }])
})

test('AppImage hanya dilaporkan, tidak dibatasi', () => {
  const r = checkSizes([
    { name: 'Nabungo_0.1.0_amd64.AppImage', bytes: 200_000_000 },
    { name: 'Nabungo-0.1.0-1.x86_64.rpm', bytes: 9_000_000 },
  ])
  assert.equal(r.ok, true)
  assert.deepEqual(r.rows[0], { name: 'Nabungo_0.1.0_amd64.AppImage', bytes: 200_000_000, limit: null, ok: true })
})

test('tanpa installer berbatas = gagal (folder salah)', () => {
  assert.equal(checkSizes([{ name: 'Nabungo_0.1.0_amd64.AppImage', bytes: 1 }]).ok, false)
  assert.equal(checkSizes([]).ok, false)
})

test('setup.exe NSIS dikenali, file lain diabaikan', () => {
  const r = checkSizes([
    { name: 'Nabungo_0.1.0_x64-setup.exe', bytes: 20_000_000 },
    { name: 'nabungo-desktop', bytes: 99_000_000 },
    { name: 'Nabungo.desktop', bytes: 300 },
  ])
  assert.equal(r.rows.length, 1)
  assert.equal(r.ok, false)
})
