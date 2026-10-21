import type { PayeeUsageDto, TagUsageDto } from '@/ipc'

export type LabelKind = 'tag' | 'payee'
export type Label = TagUsageDto | PayeeUsageDto

/** Nama tampilan: tag selalu berawalan `#`. */
export const labelText = (kind: LabelKind, name: string) => (kind === 'tag' ? `#${name}` : name)
