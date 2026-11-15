import { describe, expect, it } from 'vitest'
import type { Block } from '../ipc/types/Block'
import type { Doc } from '../ipc/types/Doc'
import { bookClasses, isBookBlank } from './composition'

const p = (text: string): Block => ({ type: 'Paragraph', runs: [{ text }] })
const h = (text: string): Block => ({ type: 'Heading', level: 1, runs: [{ text }] })
const li = (text: string): Block => ({ type: 'ListItem', depth: 0, runs: [{ text }] })
const q = (text: string): Block => ({ type: 'Quote', runs: [{ text }] })
const img: Block = { type: 'Image', id: 1 }

const doc = (blocks: Block[], chapters: number[] = [], composition: Doc['composition'] = 'Book'): Doc => ({
  title: 't',
  initial_char_index: 0,
  composition,
  chapters,
  blocks,
})

describe('bookClasses', () => {
  it('Text no lleva ninguna clase', () => {
    const classes = bookClasses(doc([h('T'), p(' '), p('Uno.'), img, li('a')], [], 'Text'))
    expect(classes).toEqual(['', '', '', '', ''])
  })

  it('Book: sangría, separación, imagen, listas y blancos', () => {
    const blocks = [h('Título'), p(' '), p('Uno.'), p('Dos.'), img, p('Tres.'), li('a'), li('b'), q('cita'), p('Fin.')]
    expect(bookClasses(doc(blocks))).toEqual([
      '',
      'blank',
      'sep',
      'sep indent',
      'sep',
      'sep',
      'sep',
      '',
      'sep',
      'sep indent',
    ])
  })

  it('capítulo en 3: lleva chapter sin sep ni indent, y el blanco previo es blank', () => {
    const blocks = [p('a'), p('b'), p('c'), p(''), p('d'), p('e')]
    expect(bookClasses(doc(blocks, [3]))).toEqual(['', 'sep indent', 'sep indent', 'blank', 'chapter', 'sep indent'])
  })

  it('dos capítulos seguidos con blancos: chapter una sola vez, en el primer no blanco de su rango', () => {
    const blocks = [p('a'), p(' '), p(''), p('d')]
    expect(bookClasses(doc(blocks, [1, 3]))).toEqual(['', 'blank', 'blank', 'chapter'])
  })

  it('un capítulo sin ningún bloque no blanco no lleva chapter', () => {
    const blocks = [p('a'), p(' '), p('c')]
    expect(bookClasses(doc(blocks, [1, 2]))).toEqual(['', 'blank', 'chapter'])
  })

  it('U+00A0 es blanco y "• " no', () => {
    expect(isBookBlank(p(' '))).toBe(true)
    expect(isBookBlank(p('• '))).toBe(false)
    expect(isBookBlank({ type: 'Paragraph', runs: [] })).toBe(true)
    expect(isBookBlank(img)).toBe(false)
    expect(bookClasses(doc([p(' '), p('• '), p('x')]))).toEqual(['blank', '', 'sep indent'])
  })
})
