import { describe, it, expect } from 'vitest'
import { DARK, LIGHT, toggledTheme } from './theme'

describe('toggledTheme', () => {
  it.each<[string, typeof LIGHT]>([
    ['#ffffff', DARK],
    ['#1e1e1e', LIGHT],
    ['#000000', LIGHT],
    ['#f4ecd8', DARK],
    ['#2b2b40', LIGHT],
  ])('fondo %s → %j', (background, expected) => {
    expect(toggledTheme(background)).toEqual(expected)
  })
})
