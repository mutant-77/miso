import { describe, it, expect, vi } from 'vitest'
import { scrollByWheel, wheelZoom } from './wheel'

const wheel = (deltaY: number, ctrlKey = false) =>
  ({ ctrlKey, deltaX: 0, deltaY, deltaMode: 0, preventDefault: vi.fn() }) as unknown as WheelEvent

describe('wheelZoom', () => {
  it('Ctrl+rueda arriba amplía y abajo reduce', () => {
    expect(wheelZoom(wheel(-100, true))).toBe('zoomIn')
    expect(wheelZoom(wheel(100, true))).toBe('zoomOut')
  })

  it('sin Ctrl o sin desplazamiento vertical no escala', () => {
    expect(wheelZoom(wheel(-100))).toBeNull()
    expect(wheelZoom(wheel(0, true))).toBeNull()
  })
})

describe('scrollByWheel', () => {
  it('desplaza en horizontal', () => {
    const viewport = { scrollLeft: 0, clientWidth: 1000 } as HTMLElement
    scrollByWheel(wheel(100), viewport)
    expect(viewport.scrollLeft).toBe(100)
  })

  it('con Ctrl no desplaza ni cancela el evento', () => {
    const viewport = { scrollLeft: 0, clientWidth: 1000 } as HTMLElement
    const e = wheel(100, true)
    scrollByWheel(e, viewport)
    expect(viewport.scrollLeft).toBe(0)
    expect(e.preventDefault).not.toHaveBeenCalled()
  })
})
