import { describe, expect, it } from 'vitest'
import type { Doc } from '../ipc/types/Doc'
import { SHY, detectLang, hyphenate } from './hyphenate'

const doc = (...lines: string[]): Doc => ({
  title: 't',
  initial_char_index: 0,
  composition: 'Text',
  chapters: [],
  blocks: lines.map((text) => ({ type: 'Paragraph' as const, runs: [{ text }] })),
})

const en = 'She heard her father now putting on the heavy pots of water, and then watched him cross the yard to the barn.'
const es = 'En un lugar de la Mancha, de cuyo nombre no quiero acordarme, no ha mucho tiempo que vivía un hidalgo de los de lanza.'

describe('detectLang', () => {
  it('inglés', () => expect(detectLang(doc(en, en))).toBe('en'))
  it('español', () => expect(detectLang(doc(es, es))).toBe('es'))
  it('sin muestra suficiente: null', () => expect(detectLang(doc('Hola mundo.'))).toBeNull())
  it('mezcla sin mayoría clara: null', () => expect(detectLang(doc(en, es))).toBeNull())
})

describe('hyphenate', () => {
  it('parte con guiones blandos según el idioma', () => {
    expect(hyphenate('beautiful', 'en')).toContain(SHY)
    expect(hyphenate('electroencefalografista', 'es')).toContain(SHY)
  })

  it('sólo añade guiones blandos: quitarlos devuelve el texto', () => {
    for (const [text, lang] of [[en, 'en'], [es, 'es']] as const) {
      expect(hyphenate(text, lang).split(SHY).join('')).toBe(text)
    }
    expect(hyphenate('a < b > c --flag', 'en').split(SHY).join('')).toBe('a < b > c --flag')
  })

  it('sin idioma no parte', () => expect(hyphenate('electroencefalografista', null)).toBe('electroencefalografista'))
})
