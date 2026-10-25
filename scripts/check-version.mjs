// Versi di tauri.conf.json, Cargo.toml ([workspace.package]), dan package.json harus sama;
// tag rilis harus `v<versi>`.
// Pakai: node scripts/check-version.mjs [tag]   (exit 1 bila tidak cocok)

import { readFileSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'

function cargoVersion(toml) {
  const block = toml.split(/^\[workspace\.package\]\s*$/m)[1]?.split(/^\[/m)[0] ?? ''
  return block.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null
}

/**
 * @param {string | null} tag
 * @param {{tauriConf: string, cargoToml: string, packageJson: string}} files
 * @returns {string[]} pesan error; kosong = cocok
 */
export function checkVersion(tag, files) {
  const versions = {
    'src-tauri/tauri.conf.json': JSON.parse(files.tauriConf).version ?? null,
    'Cargo.toml [workspace.package]': cargoVersion(files.cargoToml),
    'package.json': JSON.parse(files.packageJson).version ?? null,
  }
  const errors = []
  const expected = versions['src-tauri/tauri.conf.json']
  for (const [file, v] of Object.entries(versions)) {
    if (v === null) errors.push(`${file}: versi tidak ditemukan`)
    else if (v !== expected) errors.push(`${file}: ${v} ≠ ${expected} (src-tauri/tauri.conf.json)`)
  }
  if (tag && tag !== `v${expected}`) errors.push(`tag ${tag} ≠ v${expected} (versi di file)`)
  return errors
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const root = fileURLToPath(new URL('..', import.meta.url))
  const read = (p) => readFileSync(root + p, 'utf8')
  const errors = checkVersion(process.argv[2] || null, {
    tauriConf: read('src-tauri/tauri.conf.json'),
    cargoToml: read('Cargo.toml'),
    packageJson: read('package.json'),
  })
  for (const e of errors) console.error(e)
  if (errors.length === 0) console.log('versi cocok')
  process.exit(errors.length ? 1 : 0)
}
