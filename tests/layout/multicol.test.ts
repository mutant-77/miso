import { describe, it, expect } from 'vitest'

describe('multicolumna (premisa D1)', () => {
  it('el texto desborda en columnas horizontales y las líneas caen en columnas distintas', () => {
    const container = document.createElement('div')
    container.style.cssText =
      'height: 200px; width: 400px; column-width: 100px; column-fill: auto; overflow-x: auto; margin: 0; font: 16px monospace;'
    const paragraph = document.createElement('p')
    paragraph.style.margin = '0'
    const text = document.createTextNode('palabra '.repeat(2000))
    paragraph.appendChild(text)
    container.appendChild(paragraph)
    document.body.appendChild(container)

    try {
      expect(container.scrollWidth).toBeGreaterThan(container.clientWidth)

      const range = document.createRange()
      range.selectNodeContents(text)
      const lefts = new Set(Array.from(range.getClientRects(), (rect) => Math.round(rect.left)))
      expect(lefts.size).toBeGreaterThanOrEqual(2)
    } finally {
      container.remove()
    }
  })
})

const LH = 24
const PAD_TOP = 20
const PAD_LEFT = 15
const PITCH = 210
// Con 640px Chromium estira las columnas (paso 223,3 px); 600px da 3 columnas de 180 px exactas.
const CONTENEDOR =
  'height: 300px; width: 600px; column-width: 180px; column-gap: 30px; column-fill: auto; overflow: hidden; padding: 20px 15px; box-sizing: content-box; font: 16px/24px serif;'

const SEP = (i: number) => (i === 0 ? '' : 'margin-block-start: 0.5lh;')

function parrafo(texto: string, estilo = ''): HTMLParagraphElement {
  const p = document.createElement('p')
  p.style.cssText = `margin: 0; ${estilo}`
  p.textContent = texto
  return p
}

function libro(n: number, estilo: (i: number) => string): HTMLParagraphElement[] {
  return Array.from({ length: n }, (_, i) => parrafo('palabra '.repeat(1 + (i % 12)), estilo(i)))
}

function montar(ps: HTMLElement[]): HTMLDivElement {
  const cont = document.createElement('div')
  cont.style.cssText = CONTENEDOR
  cont.append(...ps)
  document.body.append(cont)
  return cont
}

function medidas(cont: HTMLElement) {
  const r = cont.getBoundingClientRect()
  return { top: r.top + PAD_TOP, left: r.left + PAD_LEFT }
}

type Caja = { left: number; top: number }
const cajas = (p: HTMLElement) => Array.from(p.getClientRects())
const cajasDe = (ps: HTMLElement[]) => ps.reduce<Caja[]>((acc, p) => acc.concat(cajas(p)), [])
// Borde del contenido: el padding superior no forma parte del texto y se suma al top.
const cuerpo = (p: HTMLElement): Caja[] => {
  const pad = parseFloat(getComputedStyle(p).paddingBlockStart)
  return cajas(p).map((r, i) => ({ left: r.left, top: i === 0 ? r.top + pad : r.top }))
}
const cuerpoDe = (ps: HTMLElement[]) => ps.reduce<Caja[]>((acc, p) => acc.concat(cuerpo(p)), [])
const primera = <T extends Caja>(rs: T[]): T => rs.reduce((a, b) => (b.left < a.left ? b : a))
const colOf = (r: Caja, left: number) => Math.round((r.left - left) / PITCH)

function cerca(real: number, esperado: number, msg: string) {
  expect(Math.abs(real - esperado), `${msg}: real ${real}, esperado ${esperado}`).toBeLessThanOrEqual(1)
}

function corteLimpio(): number {
  const ps = libro(200, SEP)
  const cont = montar(ps)
  try {
    const m = medidas(cont)
    for (let i = 0; i < ps.length - 1; i++) {
      const a = cajas(ps[i])
      if (a.length === 1 && colOf(a[0], m.left) === 0 && colOf(primera(cajas(ps[i + 1])), m.left) === 1) {
        return i
      }
    }
    throw new Error('ningún corte limpio entre párrafos')
  } finally {
    cont.remove()
  }
}

describe('composición de libro (premisas Chromium)', () => {
  it('(a) truncado: tras un salto de columna no forzado el margen superior no deja hueco', () => {
    const ps = libro(200, SEP)
    const cont = montar(ps)
    try {
      const m = medidas(cont)
      const todas = cuerpoDe(ps)
      const maxCol = Math.max(...todas.map((r) => colOf(r, m.left)))
      expect(maxCol + 1).toBeGreaterThanOrEqual(5)
      cerca(Math.min(...todas.filter((r) => colOf(r, m.left) === 1).map((r) => r.left)), m.left + PITCH, 'paso')
      for (let k = 1; k <= maxCol; k++) {
        const enK = todas.filter((r) => colOf(r, m.left) === k)
        expect(enK.length, `columna ${k} vacía`).toBeGreaterThan(0)
        cerca(Math.min(...enK.map((r) => r.top)), m.top, `columna ${k}`)
      }
    } finally {
      cont.remove()
    }
  })

  it('(b-1) salto forzado: el párrafo 120 abre columna y con margen 0 queda en el borde', () => {
    const ps = libro(200, (i) => SEP(i) + (i === 120 ? 'break-before: column; margin-block-start: 0;' : ''))
    const cont = montar(ps)
    try {
      const m = medidas(cont)
      const ult119 = cajas(ps[119]).reduce((a, b) => (b.left > a.left ? b : a))
      const p120 = primera(cajas(ps[120]))
      expect(colOf(p120, m.left), 'columna de p120 frente a la de p119').toBeGreaterThan(colOf(ult119, m.left))
      cerca(p120.top, m.top, 'top de p120')
    } finally {
      cont.remove()
    }
  })

  it('(b-2) salto forzado con margen 0.5lh: el margen se conserva (+12 px)', () => {
    // Tras un salto forzado el margen se conserva: por eso C6 anula el margen en .chapter.
    const ps = libro(200, (i) => SEP(i) + (i === 120 ? 'break-before: column; margin-block-start: 0.5lh;' : ''))
    const cont = montar(ps)
    try {
      const m = medidas(cont)
      const p120 = primera(cajas(ps[120]))
      cerca(p120.top, m.top + 12, 'top de p120 con margen 0.5lh')
    } finally {
      cont.remove()
    }
  })

  it('(c) sonda del ancla: en la sangría y a la izquierda de un centrado, caretPositionFromPoint da el inicio', () => {
    const p0 = parrafo('palabra '.repeat(3))
    const pInd = parrafo('palabra '.repeat(20), 'text-indent: 1.5em; break-before: column;')
    const pCen = parrafo('Capítulo', 'text-align: center; break-before: column;')
    const resto = parrafo('palabra '.repeat(200))
    const cont = montar([p0, pInd, pCen, resto])
    try {
      const m = medidas(cont)
      const casos: Array<[number, HTMLParagraphElement]> = [
        [1, pInd],
        [2, pCen],
      ]
      for (const [k, p] of casos) {
        expect(colOf(primera(cajas(p)), m.left), `columna de la caja ${k}`).toBe(k)
      }
      for (const [k, p] of casos) {
        cont.scrollLeft = k * PITCH
        cerca(primera(cajas(p)).left, m.left, `caja de la columna ${k} tras scroll`)
        const pos = document.caretPositionFromPoint(m.left + 1, m.top + LH / 2)
        expect(pos, `caret en columna ${k}`).not.toBeNull()
        expect(pos?.offsetNode).toBe(p.firstChild)
        expect(pos?.offset).toBe(0)
      }
    } finally {
      cont.remove()
    }
  })

  it('(d-1) blanco de alto 0 al final de columna: la columna siguiente empieza en el borde', () => {
    const i = corteLimpio()
    const ps = libro(200, SEP)
    const lista = [...ps.slice(0, i + 1), parrafo(' ', 'height: 0; line-height: 0;'), ...ps.slice(i + 1)]
    const cont = montar(lista)
    try {
      const m = medidas(cont)
      const blanco = primera(cajas(lista[i + 1]))
      const todas = cuerpoDe(lista)
      const enCol1 = todas.filter((r) => colOf(r, m.left) === 1)
      expect(colOf(blanco, m.left), 'blanco en columna 0').toBe(0)
      expect(enCol1.length, 'hay columna 1').toBeGreaterThan(0)
      const top1 = Math.min(...enCol1.map((r) => r.top))
      cerca(top1, m.top, 'primera caja de la columna 1')
    } finally {
      cont.remove()
    }
  })

  it('(d-2) blanco de alto 0 seguido de salto forzado: no aparece columna vacía', () => {
    const i = corteLimpio()
    const ps = libro(200, SEP)
    const bloque = parrafo('palabra '.repeat(8), 'break-before: column; margin-block-start: 0;')
    const lista = [
      ...ps.slice(0, i + 1),
      parrafo(' ', 'height: 0; line-height: 0;'),
      bloque,
      ...ps.slice(i + 2),
    ]
    const cont = montar(lista)
    try {
      const m = medidas(cont)
      const cols = cajasDe(lista).map((r) => colOf(r, m.left))
      const usadas = new Set(cols)
      const maxCol = Math.max(...cols)
      expect(colOf(primera(cajas(bloque)), m.left), 'bloque con salto en columna 1').toBe(1)
      for (let k = 0; k <= maxCol; k++) {
        expect(usadas.has(k), `columna ${k} vacía`).toBe(true)
      }
    } finally {
      cont.remove()
    }
  })
})
