// Facade IPC: satu-satunya pintu UI ke backend. Komponen tidak memakai `commands` langsung.
import { isTauri } from '@tauri-apps/api/core'
import { commands, type AppInfo, type IpcError, type LocaleDto, type ThemeDto, type UiSettingsDto } from './bindings'
import { browserCommands } from './browser'

export type { AppInfo, ErrorCode, IpcError, LocaleDto, ThemeDto, UiSettingsDto } from './bindings'

/** Error domain dari backend; `error.code` diterjemahkan oleh UI. */
export class IpcFailure extends Error {
  readonly error: IpcError

  constructor(error: IpcError) {
    super(error.entity ? `${error.code}: ${error.entity}` : error.code)
    this.name = 'IpcFailure'
    this.error = error
  }
}

type Result<T> = { status: 'ok'; data: T } | { status: 'error'; error: IpcError }

function api(): typeof browserCommands {
  return isTauri() ? commands : browserCommands
}

async function unwrap<T>(p: Promise<Result<T>>): Promise<T> {
  const r = await p
  if (r.status === 'error') throw new IpcFailure(r.error)
  return r.data
}

export function appInfo(): Promise<AppInfo> {
  return api().appInfo()
}

export function getUiSettings(): Promise<UiSettingsDto> {
  return unwrap(api().getUiSettings())
}

export async function setTheme(theme: ThemeDto): Promise<void> {
  await unwrap(api().setTheme(theme))
}

export async function setLocale(locale: LocaleDto): Promise<void> {
  await unwrap(api().setLocale(locale))
}

export function parseAmount(input: string): Promise<number> {
  return unwrap(api().parseAmount(input))
}
