import { SHY } from './hyphenate'
import { blockAt } from './textIndex'

// El DOM lleva guiones blandos que el índice de carácter no cuenta: estas dos funciones traducen.
function modelOffset(s: string, dom: number): number {
  let n = dom
  for (let i = 0; i < dom; i++) if (s[i] === SHY) n--
  return n
}

function domOffset(s: string, model: number, end: boolean): number {
  let i = 0
  for (let c = 0; c < model; i++) if (s[i] !== SHY) c++
  if (!end) while (s[i] === SHY) i++
  return i
}

function padding(columns: HTMLElement) {
  const cs = getComputedStyle(columns)
  return { left: parseFloat(cs.paddingLeft), top: parseFloat(cs.paddingTop), line: parseFloat(cs.lineHeight) }
}

function textBefore(p: Element, node: Node): number {
  const walker = document.createTreeWalker(p, NodeFilter.SHOW_TEXT)
  let n = 0
  for (let t = walker.nextNode(); t && t !== node; t = walker.nextNode()) n += modelOffset(t.textContent!, t.textContent!.length)
  return n
}

export function firstVisibleCharIndex(viewport: HTMLElement, columns: HTMLElement): number | null {
  const vp = viewport.getBoundingClientRect()
  const pad = padding(columns)
  const pos = document.caretPositionFromPoint(vp.left + pad.left + 1, vp.top + pad.top + pad.line / 2)
  if (!pos) return null
  const node = pos.offsetNode
  const el = node.nodeType === Node.TEXT_NODE ? node.parentElement : (node as Element)
  const p = el?.closest<HTMLElement>('p[data-char-start]')
  if (!p || !columns.contains(p)) return null
  const start = Number(p.dataset.charStart)
  return node.nodeType === Node.TEXT_NODE ? start + textBefore(p, node) + modelOffset(node.textContent!, pos.offset) : start
}

export function nodeAt(p: HTMLElement, offset: number, end = false): { node: Text; offset: number } | null {
  const walker = document.createTreeWalker(p, NodeFilter.SHOW_TEXT)
  for (let t = walker.nextNode(); t; t = walker.nextNode()) {
    const s = t.textContent!
    const len = modelOffset(s, s.length)
    if (end ? offset <= len : offset < len) return { node: t as Text, offset: domOffset(s, offset, end) }
    offset -= len
  }
  return null
}

function charRect(p: HTMLElement, offset: number): DOMRect | undefined {
  const n = nodeAt(p, offset)
  if (n) {
    const range = document.createRange()
    range.setStart(n.node, n.offset)
    range.setEnd(n.node, n.offset + 1)
    // Tras una palabra partida, el primer rectángulo es el guion de la línea anterior; el del carácter es el último.
    const rects = range.getClientRects()
    return rects[rects.length - 1] ?? range.getBoundingClientRect()
  }
  const rects = p.getClientRects()
  return rects[rects.length - 1]
}

export function columnOfCharIndex(
  i: number,
  starts: number[],
  viewport: HTMLElement,
  columns: HTMLElement,
  pitch: number,
): number {
  const k = blockAt(starts, i)
  if (k === -1) return 0
  const ps = columns.querySelectorAll<HTMLElement>('p[data-char-start]')
  const p = ps[k]
  const rect = p && charRect(p, i - starts[k])
  if (!rect) return 0
  const vp = viewport.getBoundingClientRect()
  return Math.max(0, Math.floor((rect.left - vp.left + viewport.scrollLeft - padding(columns).left + 1) / pitch))
}
