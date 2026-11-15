const LINE_PX = 16

type WheelInput = Pick<WheelEvent, 'ctrlKey' | 'deltaX' | 'deltaY' | 'deltaMode'>

// Ctrl+rueda escala la fuente (como Ctrl+= / Ctrl+−); no desplaza.
export function wheelZoom({ ctrlKey, deltaY }: WheelInput): 'zoomIn' | 'zoomOut' | null {
  if (!ctrlKey || deltaY === 0) return null
  return deltaY < 0 ? 'zoomIn' : 'zoomOut'
}

export function scrollByWheel(e: WheelEvent, viewport: HTMLElement): void {
  if (e.ctrlKey || Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return
  const unit = e.deltaMode === 1 ? LINE_PX : e.deltaMode === 2 ? viewport.clientWidth : 1
  viewport.scrollLeft += e.deltaY * unit
  e.preventDefault()
}
