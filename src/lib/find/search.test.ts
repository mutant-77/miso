import { describe, it, expect } from 'vitest'
import { findAll, firstAtOrAfter, type Match } from './search'

describe('search', () => {
  it('no distingue mayúsculas', () => {
    const matches = findAll('Hola HOLA hola', 'hola')
    expect(matches).toHaveLength(3)
    expect(matches[0].start).toBe(0)
    expect(matches[1].start).toBe(5)
    expect(matches[2].start).toBe(10)
  })

  it('acentos distintos no casan', () => {
    expect(findAll('canción', 'cancion')).toHaveLength(0)
  })

  it('los literales ., (, \\ se tratan como caracteres', () => {
    let matches = findAll('a.b axb', 'a.b')
    expect(matches).toHaveLength(1)
    expect(matches[0].start).toBe(0)

    matches = findAll('f(x)', '(')
    expect(matches).toHaveLength(1)
    expect(matches[0].start).toBe(1)

    matches = findAll('a\\b', '\\')
    expect(matches).toHaveLength(1)
    expect(matches[0].start).toBe(1)
  })

  it('los índices son UTF-16 (emoji)', () => {
    const matches = findAll('😀ab😀ab', 'ab')
    expect(matches).toHaveLength(2)
    expect(matches[0].start).toBe(2)
    expect(matches[1].start).toBe(6)
  })

  it('un salto de línea en el texto corta la coincidencia', () => {
    expect(findAll('ab\ncd', 'bc')).toHaveLength(0)
  })

  it('un término vacío devuelve una lista vacía', () => {
    expect(findAll('hola', '')).toHaveLength(0)
  })

  describe('firstAtOrAfter', () => {
    it('from exactamente igual al start de una coincidencia', () => {
      const matches: Match[] = [{ start: 0, end: 2 }, { start: 5, end: 7 }, { start: 10, end: 12 }]
      expect(firstAtOrAfter(matches, 5)).toBe(1)
    })

    it('from intermedio: devuelve la siguiente', () => {
      const matches: Match[] = [{ start: 0, end: 2 }, { start: 5, end: 7 }]
      expect(firstAtOrAfter(matches, 3)).toBe(1)
    })

    it('más allá de la última: vuelta al 0', () => {
      const matches: Match[] = [{ start: 0, end: 2 }]
      expect(firstAtOrAfter(matches, 100)).toBe(0)
    })

    it('lista vacía: -1', () => {
      expect(firstAtOrAfter([], 0)).toBe(-1)
    })
  })
})
