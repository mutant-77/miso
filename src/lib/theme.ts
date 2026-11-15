export const LIGHT = { background: '#ffffff', foreground: '#000000' }
export const DARK = { background: '#1e1e1e', foreground: '#d4d4d4' }

function isDark(hex: string): boolean {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16))
  return 0.299 * r + 0.587 * g + 0.114 * b < 128
}

export function toggledTheme(background: string) {
  return isDark(background) ? LIGHT : DARK
}
