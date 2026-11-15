import type { Block } from '../ipc/types/Block'
import type { Doc } from '../ipc/types/Doc'
import type { Run } from '../ipc/types/Run'

export function runsOf(block: Block): Run[] {
  return block.type === 'Image' ? [] : block.runs
}

function blockLength(block: Block): number {
  return runsOf(block).reduce((n, run) => n + run.text.length, 0)
}

export function blockStarts(doc: Doc): number[] {
  const starts: number[] = []
  let pos = 0
  for (const block of doc.blocks) {
    starts.push(pos)
    pos += blockLength(block) + 1
  }
  return starts
}

export function flatText(doc: Doc): string {
  return doc.blocks.map((block) => runsOf(block).map((run) => run.text).join('')).join('\n')
}

export function flatLength(doc: Doc): number {
  if (doc.blocks.length === 0) return 0
  return doc.blocks.reduce((n, block) => n + blockLength(block), 0) + doc.blocks.length - 1
}

export function blockAt(starts: number[], i: number): number {
  let lo = 0
  let hi = starts.length - 1
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1
    if (starts[mid] <= i) lo = mid
    else hi = mid - 1
  }
  return starts.length === 0 ? -1 : lo
}
