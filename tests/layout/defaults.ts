import type { Composition } from '../../src/lib/ipc/types/Composition'
import type { Prefs } from '../../src/lib/ipc/types/Prefs'
import '../../src/fonts.css'

// Como en main.ts: medir con la fuente definitiva.
export const fontReady = document.fonts.load('1em Literata', 'aáñЖ')

export const defaults: Prefs = {
  font_family: 'Literata',
  font_size: 14,
  column_width: 360,
  relative_inset: 3,
  vertical_inset: 20,
  line_height: 1.1,
  background: '#ffffff',
  foreground: '#000000',
  language: 'auto',
}

const sentence = 'Esta es una frase de relleno para medir las columnas del lector. '

export function longDoc(paragraphs: number, composition: Composition = 'Text') {
  return {
    title: 'largo',
    initial_char_index: 0,
    composition,
    chapters: [] as number[],
    blocks: Array.from({ length: paragraphs }, (_, i) => ({
      type: 'Paragraph' as const,
      runs: [{ text: `Párrafo ${i}. ${sentence.repeat(6)}` }],
    })),
  }
}

export function textRects(root: HTMLElement): DOMRect[] {
  const range = document.createRange()
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  const rects: DOMRect[] = []
  let top = NaN
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    range.selectNodeContents(node)
    for (const rect of range.getClientRects()) {
      // Un rectángulo en la misma línea que el anterior es el guion de una palabra partida.
      if (Math.abs(rect.top - top) >= 1) rects.push(rect)
      top = rect.top
    }
  }
  return rects
}
