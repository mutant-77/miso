import { describe, it, expect } from 'vitest'
import { actionFor, nextScale, type Action, type KeyInput } from './keyboard'

const none = { ctrlKey: false, shiftKey: false, altKey: false, metaKey: false }
const k = (key: string, mods: Partial<Omit<KeyInput, 'key'>> = {}): KeyInput => ({ key, ...none, ...mods })

describe('actionFor', () => {
  it.each<[KeyInput, Action | null]>([
    [k('ArrowLeft'), 'prev'],
    [k('ArrowRight'), 'next'],
    [k('ArrowUp'), 'prev'],
    [k('ArrowDown'), 'next'],
    [k('PageUp'), 'pagePrev'],
    [k('PageDown'), 'pageNext'],
    [k(' '), 'pageNext'],
    [k(' ', { shiftKey: true }), 'pagePrev'],
    [k('Home'), 'first'],
    [k('End'), 'last'],
    [k('o', { ctrlKey: true }), 'open'],
    [k('v', { ctrlKey: true }), 'paste'],
    [k('V', { ctrlKey: true, shiftKey: true }), 'paste'],
    [k('v', { ctrlKey: true, altKey: true }), null],
    [k('V'), null],
    [k('i', { ctrlKey: true }), 'theme'],
    [k('I', { ctrlKey: true, shiftKey: true }), 'theme'],
    [k('i'), null],
    [k('i', { ctrlKey: true, altKey: true }), null],
    [k('O', { ctrlKey: true, shiftKey: true }), 'open'],
    [k('s', { ctrlKey: true }), 'save'],
    [k('S', { ctrlKey: true, shiftKey: true }), 'save'],
    [k('s', { ctrlKey: true, altKey: true }), null],
    [k('s'), null],
    [k('=', { ctrlKey: true }), 'zoomIn'],
    [k('=', { ctrlKey: true, shiftKey: true }), 'zoomIn'],
    [k('+', { ctrlKey: true }), 'zoomIn'],
    [k('-', { ctrlKey: true }), 'zoomOut'],
    [k('0', { ctrlKey: true }), 'zoomReset'],
    [k(',', { ctrlKey: true }), 'prefs'],
    [k('ArrowRight', { ctrlKey: true }), null],
    [k('Home', { altKey: true }), null],
    [k(' ', { ctrlKey: true }), null],
    [k('ArrowLeft', { metaKey: true }), null],
    [k('ArrowUp', { altKey: true }), null],
    [k('ArrowDown', { shiftKey: true }), null],
    [k('o'), null],
    [k('o', { ctrlKey: true, altKey: true }), null],
    [k('=', { ctrlKey: true, altKey: true }), null],
    [k('='), null],
    [k(','), null],
    [k('ArrowRight', { shiftKey: true }), null],
    [k('f', { ctrlKey: true }), 'find'],
    [k('F', { ctrlKey: true, shiftKey: true }), null],
    [k('f'), null],
    [k('F3'), 'findNext'],
    [k('F3', { shiftKey: true }), 'findPrev'],
    [k('F3', { ctrlKey: true }), null],
    [k('F3', { altKey: true }), null],
    [k('F11'), 'fullscreen'],
    [k('F11', { ctrlKey: true }), null],
    [k('F11', { shiftKey: true }), null],
  ])('%j → %s', (input, expected) => {
    expect(actionFor(input)).toBe(expected)
  })
})

describe('nextScale', () => {
  it('zoomIn multiplica por 1.1', () => {
    expect(nextScale(1, 'zoomIn')).toBeCloseTo(1.1)
  })

  it('zoomOut divide entre 1.1', () => {
    expect(nextScale(1, 'zoomOut')).toBeCloseTo(1 / 1.1)
  })

  it('recorta en el tope superior', () => {
    expect(nextScale(2.9, 'zoomIn')).toBe(3)
    expect(nextScale(3, 'zoomIn')).toBe(3)
  })

  it('recorta en el tope inferior', () => {
    expect(nextScale(0.52, 'zoomOut')).toBe(0.5)
    expect(nextScale(0.5, 'zoomOut')).toBe(0.5)
  })

  it('zoomReset vuelve a 1', () => {
    expect(nextScale(2.3, 'zoomReset')).toBe(1)
  })
})
