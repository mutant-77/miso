import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { Prefs } from './types/Prefs'

export function onDocumentReady(cb: () => void): Promise<UnlistenFn> {
  return getCurrentWebviewWindow().listen('document-ready', () => cb())
}

export function onPrefsChanged(cb: (prefs: Prefs) => void): Promise<UnlistenFn> {
  return getCurrentWebviewWindow().listen<Prefs>('prefs-changed', (event) => cb(event.payload))
}
