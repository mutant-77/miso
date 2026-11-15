import { nodeAt } from '../layout/anchor'
import { blockAt } from '../layout/textIndex'
import type { Match } from './search'

export function rangeOf(columns: HTMLElement, starts: number[], m: Match): Range | null {
  const k = blockAt(starts, m.start)
  const p = k === -1 ? undefined : columns.querySelectorAll<HTMLElement>('p[data-char-start]')[k]
  if (!p) return null
  const start = nodeAt(p, m.start - starts[k])
  const end = nodeAt(p, m.end - starts[k], true)
  if (!start || !end) return null
  const range = document.createRange()
  range.setStart(start.node, start.offset)
  range.setEnd(end.node, end.offset)
  return range
}

export function showMatch(range: Range | null): void {
  if (range) CSS.highlights.set('find', new Highlight(range))
  else CSS.highlights.delete('find')
}
