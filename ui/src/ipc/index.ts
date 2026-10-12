import { invoke, isTauri } from '@tauri-apps/api/core'

// Sementara manual; akan diganti binding hasil generate tauri-specta (lihat docs/plans).
export async function appVersion(): Promise<string> {
  if (!isTauri()) return 'browser-dev'
  return invoke<string>('app_version')
}
