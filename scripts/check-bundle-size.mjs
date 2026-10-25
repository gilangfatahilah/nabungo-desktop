// Ukuran installer (PRD §8: < 15 MB). AppImage membundel WebKitGTK sendiri, jadi hanya dilaporkan.
// Pakai: node scripts/check-bundle-size.mjs <folder bundle>   (exit 1 bila melewati batas)

import { readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

export const LIMIT = 15_000_000

const LIMITED = ['.deb', '.rpm', '.msi', '-setup.exe']

/** @param {{name: string, bytes: number}[]} files */
export function checkSizes(files) {
  const rows = []
  for (const { name, bytes } of files) {
    if (LIMITED.some((s) => name.endsWith(s))) rows.push({ name, bytes, limit: LIMIT, ok: bytes < LIMIT })
    else if (name.endsWith('.AppImage')) rows.push({ name, bytes, limit: null, ok: true })
  }
  const ok = rows.some((r) => r.limit !== null) && rows.every((r) => r.ok)
  return { rows, ok }
}

function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name)
    if (e.isDirectory()) return walk(p)
    return e.isFile() ? [{ name: e.name, bytes: statSync(p).size }] : []
  })
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const dir = process.argv[2]
  if (!dir) {
    console.error('pakai: node scripts/check-bundle-size.mjs <folder bundle>')
    process.exit(2)
  }
  const { rows, ok } = checkSizes(walk(dir))
  for (const r of rows) {
    const mb = (r.bytes / 1_000_000).toFixed(2)
    const status = r.limit === null ? 'info (tanpa batas)' : r.ok ? 'ok' : `MELEBIHI ${r.limit / 1_000_000} MB`
    console.log(`${mb.padStart(8)} MB  ${r.name}  ${status}`)
  }
  if (!rows.some((r) => r.limit !== null)) console.error(`tidak ada installer (.deb/.rpm/.msi/-setup.exe) di ${dir}`)
  process.exit(ok ? 0 : 1)
}
