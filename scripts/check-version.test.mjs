import assert from 'node:assert/strict'
import { test } from 'node:test'
import { checkVersion } from './check-version.mjs'

const files = (v = { tauri: '0.1.0', cargo: '0.1.0', pkg: '0.1.0' }) => ({
  tauriConf: JSON.stringify({ productName: 'Nabungo', version: v.tauri }),
  cargoToml: `[workspace]\nmembers = []\n\n[workspace.package]\nversion = "${v.cargo}"\nedition = "2024"\n\n[dependencies]\nfoo = { version = "9.9.9" }\n`,
  packageJson: JSON.stringify({ name: 'nabungo', version: v.pkg }),
})

test('versi sama dan tag cocok', () => {
  assert.deepEqual(checkVersion('v0.1.0', files()), [])
  assert.deepEqual(checkVersion(null, files()), [])
  assert.deepEqual(checkVersion('', files()), [])
})

test('tag tidak cocok dengan versi file', () => {
  const errors = checkVersion('v1.0.0', files())
  assert.equal(errors.length, 1)
  assert.match(errors[0], /v1\.0\.0/)
  assert.match(errors[0], /0\.1\.0/)
})

test('tag tanpa awalan v ditolak', () => {
  assert.equal(checkVersion('0.1.0', files()).length, 1)
})

test('file yang berbeda disebut namanya', () => {
  const errors = checkVersion(null, files({ tauri: '0.1.0', cargo: '0.1.0', pkg: '0.2.0' }))
  assert.equal(errors.length, 1)
  assert.match(errors[0], /package\.json/)
})

test('versi Cargo diambil dari [workspace.package], bukan dependensi', () => {
  const f = files()
  f.cargoToml = f.cargoToml.replace('[workspace.package]\nversion = "0.1.0"', '[workspace.package]\nedition = "2024"')
  assert.match(checkVersion(null, f).join(), /Cargo\.toml/)
})
