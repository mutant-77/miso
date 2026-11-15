import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import type { Doc } from './types/Doc'
import type { Prefs } from './types/Prefs'

export const imageSrc = (id: number) => convertFileSrc(String(id), 'miso-img')

export function takePendingDocument(): Promise<Doc | null> {
  return invoke<Doc | null>('take_pending_document')
}

export function openDialog(): Promise<void> {
  return invoke<void>('open_dialog')
}

export function saveCopy(): Promise<void> {
  return invoke<void>('save_copy')
}

export function getPrefs(): Promise<Prefs> {
  return invoke<Prefs>('get_prefs')
}

export function setPrefs(prefs: Prefs): Promise<Prefs> {
  return invoke<Prefs>('set_prefs', { prefs })
}

export function resetPrefs(): Promise<Prefs> {
  return invoke<Prefs>('reset_prefs')
}

export function listFonts(): Promise<string[]> {
  return invoke<string[]>('list_fonts')
}

export function openFromClipboard(): Promise<void> {
  return invoke<void>('open_from_clipboard')
}

export function setPosition(charIndex: number): Promise<void> {
  return invoke<void>('set_position', { charIndex })
}
