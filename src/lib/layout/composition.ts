import type { Block } from '../ipc/types/Block'
import type { Doc } from '../ipc/types/Doc'
import { runsOf } from './textIndex'

export function isBookBlank(block: Block): boolean {
  return block.type !== 'Image' && runsOf(block).every((run) => /^\s*$/.test(run.text))
}

export function bookClasses(doc: Doc): string[] {
  const { blocks, chapters } = doc
  const out = blocks.map(() => '')
  if (doc.composition !== 'Book') return out

  const starts = new Set(chapters)
  const chapterFirst = new Set<number>()
  chapters.forEach((from, c) => {
    const to = c + 1 < chapters.length ? chapters[c + 1] : blocks.length
    for (let i = from; i < to; i++) {
      if (!isBookBlank(blocks[i])) {
        chapterFirst.add(i)
        break
      }
    }
  })

  let prev: Block | null = null
  blocks.forEach((block, i) => {
    if (starts.has(i)) prev = null
    if (isBookBlank(block)) {
      out[i] = 'blank'
      return
    }
    const cls: string[] = []
    if (chapterFirst.has(i)) cls.push('chapter')
    if (prev !== null && !(prev.type === 'ListItem' && block.type === 'ListItem')) cls.push('sep')
    if (block.type === 'Paragraph' && prev !== null && prev.type !== 'Heading' && prev.type !== 'Image') {
      cls.push('indent')
    }
    out[i] = cls.join(' ')
    prev = block
  })
  return out
}
