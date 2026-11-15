import { describe, it, expect } from 'vitest'
import { fitViewport, fullscreenColumnWidth } from './columnWidth'

describe('fitViewport', () => {
  it.each<[number, number, number]>([
    [381.6, 381.6, 381.6],
    [500, 381.6, 381.6],
    [763.2, 381.6, 763.2],
    [1000, 381.6, 763.2],
    [1144.7, 381.6, 763.2],
    [1144.8, 381.6, 1144.8],
    [300, 381.6, 300],
    [1000, 200, 1000],
  ])('available %d, pitch %d → %d', (available, pitch, expected) => {
    expect(fitViewport(available, pitch)).toBeCloseTo(expected, 6)
  })
})

describe('fullscreenColumnWidth (regla √2)', () => {
  const inset = 3
  it.each<[number, number, number]>([
    [1920, 360, 5],
    [1000, 360, 2],
    [1100, 360, 3],
    [707, 500, 1],
    [720, 500, 2],
    [300, 360, 1],
  ])('disponible %d, preferido %d → %d columnas que llenan el ancho', (available, preferred, cols) => {
    const cw = fullscreenColumnWidth(available, preferred, inset)
    expect(Math.round(available / (cw * (1 + inset / 50)))).toBe(cols)
    expect(cols * (cw + 2 * (inset / 100) * cw)).toBeCloseTo(available, 6)
  })
})
