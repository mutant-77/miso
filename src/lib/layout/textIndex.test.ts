import { describe, it, expect } from 'vitest'
import unicodeJson from '../../../tests/fixtures/unicode.doc.json'
import type { Doc } from '../ipc/types/Doc'
import { blockAt, blockStarts, flatLength, flatText } from './textIndex'

const unicode = unicodeJson as Doc

describe('textIndex', () => {
  it('flatLength de unicode.doc.json es K = 25 (paridad con el test de Rust)', () => {
    expect(flatLength(unicode)).toBe(25)
  })

  it('un documento vacío mide 0 y no tiene inicios de bloque', () => {
    const empty: Doc = { title: 'vacio.txt', initial_char_index: 0, composition: 'Text', chapters: [], blocks: [] }
    expect(flatLength(empty)).toBe(0)
    expect(blockStarts(empty)).toEqual([])
  })

  it('blockStarts cuenta el separador de 1 unidad entre bloques', () => {
    expect(blockStarts(unicode)).toEqual([0, 8, 11, 20, 21, 25])
  })

  it('flatText(unicode).length === flatLength(unicode) (= 25)', () => {
    expect(flatText(unicode).length).toBe(flatLength(unicode))
    expect(flatLength(unicode)).toBe(25)
  })

  it('una imagen mide 0 unidades y el separador cuenta 1 como entre cualquier bloque', () => {
    const doc: Doc = {
      title: 'imagen',
      initial_char_index: 0,
      composition: 'Text',
      chapters: [],
      blocks: [
        { type: 'Paragraph', runs: [{ text: 'ab' }] },
        { type: 'Image', id: 7 },
        { type: 'Paragraph', runs: [{ text: 'c' }] },
      ],
    }
    expect(flatText(doc)).toBe('ab\n\nc')
    expect(flatLength(doc)).toBe(5)
    expect(blockStarts(doc)).toEqual([0, 3, 4])
  })

  it('blockAt devuelve el índice del último bloque con start <= i', () => {
    const starts = blockStarts(unicode)
    expect(starts).toEqual([0, 8, 11, 20, 21, 25])
    expect(blockAt(starts, 0)).toBe(0)
    expect(blockAt(starts, 7)).toBe(0)
    expect(blockAt(starts, 8)).toBe(1)
    expect(blockAt(starts, 11)).toBe(2)
    expect(blockAt(starts, 20)).toBe(3)
    expect(blockAt(starts, 21)).toBe(4)
    expect(blockAt(starts, 24)).toBe(4)
    expect(blockAt([], 0)).toBe(-1)
  })
})
