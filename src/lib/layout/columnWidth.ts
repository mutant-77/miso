// Viewport = N pasos exactos: gap = 2 × padding-inline, así el padding de los extremos completa el último paso.
export function fitViewport(available: number, pitch: number): number {
  return Math.min(available, Math.max(1, Math.floor(available / pitch + 1e-6)) * pitch)
}

// Regla √2 de Tofu (ARCHITECTURE §6): ancho de columna que hace exactos los pasos del ancho disponible.
export function fullscreenColumnWidth(available: number, preferred: number, relativeInset: number): number {
  const n = available / preferred
  const floor = Math.floor(n)
  const cols = floor === 0 ? 1 : n / floor <= 1.4142 ? floor : floor + 1
  return available / (cols * (1 + relativeInset / 50))
}
