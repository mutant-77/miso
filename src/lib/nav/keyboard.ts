export type Action =
  | 'prev'
  | 'next'
  | 'pagePrev'
  | 'pageNext'
  | 'first'
  | 'last'
  | 'open'
  | 'save'
  | 'paste'
  | 'zoomIn'
  | 'zoomOut'
  | 'zoomReset'
  | 'prefs'
  | 'theme'
  | 'find'
  | 'findNext'
  | 'findPrev'
  | 'fullscreen'

export type KeyInput = {
  key: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
}

export function actionFor({ key, ctrlKey, shiftKey, altKey, metaKey }: KeyInput): Action | null {
  if (ctrlKey && !altKey && !metaKey) {
    switch (key) {
      case 'o':
      case 'O':
        return 'open'
      case 's':
      case 'S':
        return 'save'
      case 'v':
      case 'V':
        return 'paste'
      case '=':
      case '+':
        return 'zoomIn'
      case '-':
        return 'zoomOut'
      case '0':
        return 'zoomReset'
      case ',':
        return 'prefs'
      case 'i':
      case 'I':
        return 'theme'
      case 'f':
        return 'find'
    }
  }
  if (ctrlKey || altKey || metaKey) return null
  if (key === ' ') return shiftKey ? 'pagePrev' : 'pageNext'
  if (key === 'F3') {
    return shiftKey ? 'findPrev' : 'findNext'
  }
  if (shiftKey) return null
  switch (key) {
    case 'F11':
      return 'fullscreen'
    case 'ArrowLeft':
    case 'ArrowUp':
      return 'prev'
    case 'ArrowRight':
    case 'ArrowDown':
      return 'next'
    case 'PageUp':
      return 'pagePrev'
    case 'PageDown':
      return 'pageNext'
    case 'Home':
      return 'first'
    case 'End':
      return 'last'
    default:
      return null
  }
}

export function nextScale(scale: number, action: 'zoomIn' | 'zoomOut' | 'zoomReset'): number {
  if (action === 'zoomReset') return 1
  const next = action === 'zoomIn' ? scale * 1.1 : scale / 1.1
  return Math.min(3, Math.max(0.5, next))
}
