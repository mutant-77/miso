import { columnOfCharIndex, firstVisibleCharIndex } from '../layout/anchor'

const REDUCED_MOTION = '(prefers-reduced-motion: reduce)'

export function measurePitch(columns: HTMLElement): number | null {
  let first: number | null = null
  for (const p of columns.querySelectorAll<HTMLElement>('p[data-char-start]')) {
    for (const rect of p.getClientRects()) {
      if (first === null) first = rect.left
      else if (Math.abs(rect.left - first) > 1) return rect.left - first
    }
  }
  return null
}

function motion(): ScrollBehavior {
  return matchMedia(REDUCED_MOTION).matches ? 'instant' : 'smooth'
}

export function createColumnScroller(
  viewport: HTMLElement,
  columns: HTMLElement,
  starts: () => number[],
  initialAnchor: number,
  onAnchor?: (i: number) => void,
) {
  let pitch: number | null = null
  let k = 0
  let anchor = initialAnchor
  // Posición en la que el ancla es válida: un scrollend que acaba aquí no es del usuario (H5).
  let anchoredLeft = 0

  const maxScroll = () => viewport.scrollWidth - viewport.clientWidth
  const kMax = (p: number) => Math.floor((maxScroll() + 1) / p)

  function reveal(i: number, behavior: ScrollBehavior = motion()): void {
    if (pitch === null) return
    const col = columnOfCharIndex(i, starts(), viewport, columns, pitch)
    const vis = visibleColumns()
    if (k <= col && col <= k + vis - 1) return
    goTo(col - Math.floor((vis - 1) / 2), behavior)
  }

  function goTo(target: number, behavior: ScrollBehavior = motion()) {
    if (pitch === null) return
    k = Math.min(Math.max(target, 0), kMax(pitch))
    viewport.scrollTo({ left: Math.min(Math.round(k * pitch), maxScroll()), behavior })
  }

  function go(delta: number) {
    goTo(k + delta)
  }

  function first() {
    goTo(0)
  }

  function last() {
    if (pitch === null) return
    k = kMax(pitch)
    viewport.scrollTo({ left: maxScroll(), behavior: motion() })
  }

  function visibleColumns(): number {
    if (pitch === null) return 1
    const gap = parseFloat(getComputedStyle(columns).columnGap)
    return Math.max(1, Math.floor((viewport.clientWidth + gap) / pitch))
  }

  function sync() {
    if (pitch === null) return
    const max = maxScroll()
    k =
      viewport.scrollLeft >= max - 1
        ? kMax(pitch)
        : Math.min(Math.round(viewport.scrollLeft / pitch), kMax(pitch))
    if (viewport.scrollLeft === anchoredLeft) return
    const previous = anchor
    anchor = firstVisibleCharIndex(viewport, columns) ?? anchor
    anchoredLeft = viewport.scrollLeft
    if (anchor !== previous) onAnchor?.(anchor)
  }

  function remeasure() {
    pitch = measurePitch(columns)
    if (pitch === null) k = 0
  }

  function restore() {
    remeasure()
    if (pitch !== null) goTo(columnOfCharIndex(anchor, starts(), viewport, columns, pitch), 'instant')
    anchoredLeft = viewport.scrollLeft
  }

  return { go, goTo, first, last, visibleColumns, sync, restore, reveal, anchor: () => anchor }
}
