import { beforeAll, describe, it, expect } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import type { Block } from '../../src/lib/ipc/types/Block'
import type { Doc } from '../../src/lib/ipc/types/Doc'
import type { Prefs } from '../../src/lib/ipc/types/Prefs'
import type { Match } from '../../src/lib/find/search'
import Columns from '../../src/components/Columns.svelte'
import { measurePitch } from '../../src/lib/nav/columnScroll'
import { blockStarts, flatLength, flatText, runsOf } from '../../src/lib/layout/textIndex'
import { columnOfCharIndex, firstVisibleCharIndex } from '../../src/lib/layout/anchor'
import { findAll } from '../../src/lib/find/search'
import { rangeOf } from '../../src/lib/find/highlight'
import { SHY } from '../../src/lib/layout/hyphenate'
import { defaults, fontReady, longDoc } from './defaults'

// Como images.test.ts: la URL la da el stub y la imagen llega de forma asíncrona.
const sources = new Map<number, string>()
const BROKEN = 'data:image/png;base64,AAAA'
const tauri = window as unknown as { __TAURI_INTERNALS__: unknown }
tauri.__TAURI_INTERNALS__ = { convertFileSrc: (id: string) => sources.get(Number(id)) ?? BROKEN }

async function image(id: number, w: number, h: number): Promise<Block> {
  const canvas = document.createElement('canvas')
  canvas.width = w
  canvas.height = h
  canvas.getContext('2d')!.fillRect(0, 0, w, h)
  const blob = await new Promise<Blob>((resolve) => canvas.toBlob((b) => resolve(b!), 'image/png'))
  sources.set(id, URL.createObjectURL(blob))
  return { type: 'Image', id }
}

// Títulos cada 15 párrafos (con un blanco U+00A0 detrás), blanco entre párrafos cada 25, imagen cada 40, capítulo en cada 60 (n = 60k + 7, un párrafo).
async function libro(): Promise<Doc> {
  const src = longDoc(600, 'Book').blocks
  const blocks: Block[] = []
  const chapters: number[] = []
  for (let n = 0; n < src.length; n++) {
    if (n % 60 === 7) chapters.push(blocks.length)
    if (n % 15 === 0) {
      blocks.push(
        { type: 'Heading', level: 2, runs: [{ text: `Título ${n}:` }] },
        { type: 'Paragraph', runs: [{ text: ' ' }] },
      )
    } else {
      blocks.push(src[n])
      // Blanco entre párrafos, como los <p>&nbsp;</p> de Calibre.
      if (n % 25 === 3) blocks.push({ type: 'Paragraph', runs: [{ text: ' ' }] })
    }
    if (n % 40 === 20) blocks.push(await image(5000 + n, 240, 180))
  }
  return { title: 'libro', initial_char_index: 0, composition: 'Book', chapters, blocks }
}

let book: Doc

beforeAll(async () => {
  await fontReady
  book = await libro()
})

type Props = { doc: Doc; prefs: Prefs; scale: number; fullscreen: boolean }
type Api = {
  goTo(k: number, behavior?: ScrollBehavior): void
  anchor(): number
  show(m: Match | null, behavior?: ScrollBehavior): void
}

const frames = (n = 3) =>
  new Promise<void>((resolve) => {
    const step = (left: number) => (left === 0 ? resolve() : requestAnimationFrame(() => step(left - 1)))
    step(n)
  })

const scrollend = (el: HTMLElement) =>
  new Promise<void>((resolve) => {
    const t = setTimeout(resolve, 500)
    el.addEventListener('scrollend', () => (clearTimeout(t), resolve()), { once: true })
  })

async function settle() {
  flushSync()
  await frames()
}

async function setup(doc: Doc) {
  const host = document.createElement('div')
  host.style.cssText = 'width: 1000px; height: 700px;'
  document.body.appendChild(host)
  const props = $state<Props>({ doc, prefs: { ...defaults }, scale: 1, fullscreen: false })
  const component = mount(Columns, { target: host, props }) as unknown as Api & Record<string, unknown>
  flushSync()
  await Promise.all(Array.from(host.querySelectorAll('img'), (img) => img.decode().catch(() => undefined)))
  await settle()
  const viewport = host.querySelector<HTMLElement>('.viewport')!
  const columns = host.querySelector<HTMLElement>('.columns')!
  return {
    host,
    props,
    component,
    viewport,
    columns,
    cleanup: async () => {
      await unmount(component)
      host.remove()
    },
  }
}

type T = Awaited<ReturnType<typeof setup>>

const isBlank = (b: Block) => b.type !== 'Image' && runsOf(b).every((r) => /^\s*$/.test(r.text))
const padTop = (p: HTMLElement) => parseFloat(getComputedStyle(p).paddingBlockStart)
const one = (p: HTMLElement): DOMRect | null => {
  const rects = p.getClientRects()
  return rects.length === 1 ? rects[0] : null
}
const textRects = (p: HTMLElement): DOMRect[] => {
  const range = document.createRange()
  range.selectNodeContents(p.firstChild!)
  return Array.from(range.getClientRects())
}
// Primer rectángulo de la primera línea (el de menor top y, en la misma línea, el de menor left).
function lineStart(rects: DOMRect[]): DOMRect {
  return rects.reduce((a, b) => (Math.abs(b.top - a.top) > 0.5 ? (b.top < a.top ? b : a) : b.left < a.left ? b : a))
}
// Primera línea del primer fragmento: un párrafo partido tiene su texto también en la columna siguiente.
function primeraLinea(p: HTMLElement): DOMRect {
  const f = p.getClientRects()[0]
  return lineStart(textRects(p).filter((r) => r.left >= f.left - 0.5 && r.left < f.right))
}

// Medidas sobre cajas de <p>, como multicol.test.ts. Columna k = base + k × paso; base se mide en cada llamada.
function geo(t: T) {
  const cs = getComputedStyle(t.columns)
  const pitch = measurePitch(t.columns)!
  const base = t.columns.getBoundingClientRect().left + parseFloat(cs.paddingLeft)
  return {
    pitch,
    ps: Array.from(t.columns.querySelectorAll<HTMLElement>('p[data-char-start]')),
    top: t.viewport.getBoundingClientRect().top + parseFloat(cs.paddingTop),
    col: (r: DOMRect) => Math.round((r.left - base) / pitch),
  }
}

function charLeft(columns: HTMLElement, index: number): number {
  const p = Array.from(columns.querySelectorAll<HTMLElement>('p[data-char-start]:not(.img)'))
    .filter((el) => Number(el.dataset.charStart) <= index)
    .pop()!
  const text = p.firstChild!
  // Posición en el DOM: el índice no cuenta los guiones blandos de la partición.
  const s = text.textContent!
  let offset = 0
  for (let c = index - Number(p.dataset.charStart); c > 0 || s[offset] === SHY; offset++) if (s[offset] !== SHY) c--
  const range = document.createRange()
  range.setStart(text, offset)
  range.setEnd(text, offset + 1)
  const rects = range.getClientRects()
  return rects[rects.length - 1].left
}

// Ancla en la primera columna: el bloque cuyo primer fragmento abre columna k ≥ 1 con sangría.
function aperturaConSangria(t: T): number {
  const g = geo(t)
  return g.ps.findIndex((p) => {
    if (!p.classList.contains('indent')) return false
    const r = p.getClientRects()[0]
    return g.col(r) >= 1 && Math.abs(r.top + padTop(p) - g.top) <= 1
  })
}

async function anclaEn(t: T, idx: number) {
  const col = columnOfCharIndex(idx, blockStarts(book), t.viewport, t.columns, geo(t).pitch)
  t.component.goTo(col, 'instant')
  await scrollend(t.viewport)
  expect(t.component.anchor()).toBe(idx)
}

describe('composición de libro (P4)', () => {
  it('(a) D9: texto de cada <p> = runs unidos, sin ::before, el índice cierra en flatLength', async () => {
    const t = await setup(book)
    try {
      const ps = geo(t).ps
      expect(ps).toHaveLength(book.blocks.length)
      expect(ps.map((p) => p.textContent!.split(SHY).join(''))).toEqual(
        book.blocks.map((block) => runsOf(block).map((run) => run.text).join('')),
      )
      for (const p of ps) expect(getComputedStyle(p, '::before').content).toBe('none')
      const last = ps[ps.length - 1]
      expect(Number(last.dataset.charStart) + last.textContent!.split(SHY).join('').length).toBe(flatLength(book))
    } finally {
      await t.cleanup()
    }
  })

  it('(b) separación: media línea entre párrafos, también a través de un blanco; el blanco mide 0; en Text, 0', async () => {
    const t = await setup(book)
    try {
      const g = geo(t)
      const line = (p: HTMLElement) => parseFloat(getComputedStyle(p).lineHeight)
      const sameColumn = (a: HTMLElement, b: HTMLElement) =>
        one(a) !== null && one(b) !== null && g.col(one(a)!) === g.col(one(b)!)

      const directo = book.blocks.findIndex(
        (x, i) => i > 20 && x.type === 'Paragraph' && book.blocks[i + 1].type === 'Paragraph' && sameColumn(g.ps[i], g.ps[i + 1]),
      )
      expect(directo).toBeGreaterThan(0)
      const [a, b] = [g.ps[directo], g.ps[directo + 1]]
      expect(Math.abs(one(b)!.top - one(a)!.bottom - line(b) / 2)).toBeLessThanOrEqual(0.5)

      const conBlanco = book.blocks.findIndex(
        (x, i) =>
          i > 20 &&
          x.type === 'Paragraph' &&
          isBlank(book.blocks[i + 1]) &&
          book.blocks[i + 2].type === 'Paragraph' &&
          sameColumn(g.ps[i], g.ps[i + 2]),
      )
      expect(conBlanco).toBeGreaterThan(0)
      const [c, blank, d] = [g.ps[conBlanco], g.ps[conBlanco + 1], g.ps[conBlanco + 2]]
      expect(blank.getBoundingClientRect().height).toBeLessThan(0.5)
      expect(Math.abs(one(d)!.top - one(c)!.bottom - line(d) / 2)).toBeLessThanOrEqual(0.5)
    } finally {
      await t.cleanup()
    }

    const texto = await setup(longDoc(60, 'Text'))
    try {
      const g = geo(texto)
      const i = [0, 1, 2, 3].find((k) => one(g.ps[k]) !== null && one(g.ps[k + 1]) !== null && g.col(one(g.ps[k])!) === g.col(one(g.ps[k + 1])!))
      expect(i).toBeDefined()
      expect(Math.abs(one(g.ps[i! + 1])!.top - one(g.ps[i!])!.bottom)).toBeLessThanOrEqual(0.5)
    } finally {
      await texto.cleanup()
    }
  })

  it('(c) sangría 1.5em tras párrafo; 0 tras título, tras imagen e inicio de capítulo; continuación sin sangría; título centrado', async () => {
    const t = await setup(book)
    try {
      const g = geo(t)
      const fs = (p: HTMLElement) => parseFloat(getComputedStyle(p).fontSize)
      const offsetOf = (p: HTMLElement) => primeraLinea(p).left - p.getClientRects()[0].left

      const sangria = g.ps.findIndex((p, i) => i > 50 && p.classList.contains('indent') && one(p) !== null)
      expect(sangria).toBeGreaterThan(0)
      expect(Math.abs(offsetOf(g.ps[sangria]) - 1.5 * fs(g.ps[sangria])), 'sangría').toBeLessThanOrEqual(0.5)

      const trasTitulo = book.blocks.findIndex((x, i) => i > 2 && x.type === 'Paragraph' && book.blocks[i - 2].type === 'Heading' && isBlank(book.blocks[i - 1]))
      const trasImagen = book.blocks.findIndex((x, i) => i > 0 && x.type === 'Paragraph' && book.blocks[i - 1].type === 'Image')
      const inicio = book.chapters[0]
      expect(book.blocks[inicio].type).toBe('Paragraph')
      for (const [nombre, i] of [
        ['tras título', trasTitulo],
        ['tras imagen', trasImagen],
        ['inicio de capítulo', inicio],
      ] as const) {
        expect(i, nombre).toBeGreaterThan(0)
        expect(Math.abs(offsetOf(g.ps[i])), nombre).toBeLessThanOrEqual(0.5)
      }

      const partido = g.ps.findIndex((p) => p.classList.contains('indent') && p.getClientRects().length > 1)
      expect(partido).toBeGreaterThan(0)
      const segundo = g.ps[partido].getClientRects()[1]
      const enSiguiente = textRects(g.ps[partido]).filter((r) => r.left >= segundo.left - 0.5 && r.left < segundo.right)
      expect(Math.abs(lineStart(enSiguiente).left - segundo.left), 'continuación').toBeLessThanOrEqual(0.5)

      const titulo = g.ps[0]
      const tr = textRects(titulo)
      expect(tr).toHaveLength(1)
      const caja = titulo.getClientRects()[0]
      expect(Math.abs((tr[0].left + tr[0].right) / 2 - (caja.left + caja.right) / 2), 'centro').toBeLessThanOrEqual(1)
    } finally {
      await t.cleanup()
    }
  })

  it('(d) sin hueco arriba: en las columnas 1..40 el primer bloque no blanco empieza en el borde del contenido', async () => {
    const t = await setup(book)
    try {
      const g = geo(t)
      expect((t.viewport.scrollWidth - t.viewport.clientWidth) / g.pitch).toBeGreaterThanOrEqual(40)
      // Borde del contenido: el padding superior sólo cuenta en el primer fragmento.
      const fragmentos: { r: DOMRect; top: number }[] = []
      book.blocks.forEach((b, i) => {
        if (isBlank(b)) return
        Array.from(g.ps[i].getClientRects()).forEach((r, j) => {
          fragmentos.push({ r, top: r.top + (j === 0 ? padTop(g.ps[i]) : 0) })
        })
      })
      for (let k = 1; k <= 40; k++) {
        const tops = fragmentos.filter(({ r }) => g.col(r) === k).map(({ top }) => top)
        expect(tops.length, `columna ${k} vacía`).toBeGreaterThan(0)
        expect(Math.abs(Math.min(...tops) - g.top), `columna ${k}`).toBeLessThanOrEqual(1)
      }
    } finally {
      await t.cleanup()
    }
  })

  it('(e) capítulos: columna nueva, en el borde del contenido y con el índice en esa columna', async () => {
    const t = await setup(book)
    try {
      const starts = blockStarts(book)
      for (const [c, from] of book.chapters.entries()) {
        let f = from
        while (isBlank(book.blocks[f])) f++
        let q = from - 1
        while (q > 0 && isBlank(book.blocks[q])) q--

        const g = geo(t)
        const colF = g.col(g.ps[f].getClientRects()[0])
        const ultimos = g.ps[q].getClientRects()
        const colQ = g.col(ultimos[ultimos.length - 1])
        expect(colF, `capítulo ${c}: columna frente a la anterior`).toBeGreaterThan(colQ)
        expect(Math.abs(g.ps[f].getClientRects()[0].top - g.top), `capítulo ${c}: top`).toBeLessThanOrEqual(1)

        t.component.goTo(colF, 'instant')
        expect(firstVisibleCharIndex(t.viewport, t.columns), `capítulo ${c}: primer carácter`).toBe(starts[f])
        expect(columnOfCharIndex(starts[f], starts, t.viewport, t.columns, g.pitch), `capítulo ${c}: columna`).toBe(colF)
      }
    } finally {
      await t.cleanup()
    }
  })

  describe('ancla (P4)', () => {
    const changes: [string, (p: Props, host: HTMLElement) => void][] = [
      ['font_size', (p) => (p.prefs.font_size = 18)],
      ['column_width', (p) => (p.prefs.column_width = 300)],
      ['line_height', (p) => (p.prefs.line_height = 1.6)],
      ['relative_inset', (p) => (p.prefs.relative_inset = 10)],
      ['vertical_inset', (p) => (p.prefs.vertical_inset = 60)],
      ['scale', (p) => (p.scale = 1.331)],
      ['ancho del host', (_, host) => (host.style.width = '1400px')],
      ['pantalla completa', (p) => (p.fullscreen = true)],
    ]

    async function cambiaYMide(t: T, change: (p: Props, host: HTMLElement) => void) {
      const anchor = t.component.anchor()
      change(t.props, t.host)
      await settle()
      const pitch = measurePitch(t.columns)!
      const left = t.viewport.getBoundingClientRect().left
      const x = charLeft(t.columns, anchor)
      expect(x).toBeGreaterThanOrEqual(left - 0.5)
      expect(x).toBeLessThan(left + pitch)
      expect(t.component.anchor()).toBe(anchor)
    }

    it.each(changes)('tras goTo(37) y cambiar %s, el ancla queda en la primera columna visible', async (_, change) => {
      const t = await setup(book)
      try {
        t.component.goTo(37, 'instant')
        await scrollend(t.viewport)
        expect(t.component.anchor()).toBeGreaterThan(0)
        await cambiaYMide(t, change)
      } finally {
        await t.cleanup()
      }
    })

    it.each(changes)('con el ancla al inicio de un capítulo, cambiar %s la mantiene', async (_, change) => {
      const t = await setup({ ...book, initial_char_index: blockStarts(book)[book.chapters[2]] })
      try {
        await cambiaYMide(t, change)
      } finally {
        await t.cleanup()
      }
    })

    it.each(changes)('con el ancla en un párrafo con sangría que abre columna, cambiar %s la mantiene', async (_, change) => {
      const t = await setup(book)
      try {
        const k = aperturaConSangria(t)
        expect(k).toBeGreaterThan(0)
        await anclaEn(t, blockStarts(book)[k])
        await cambiaYMide(t, change)
      } finally {
        await t.cleanup()
      }
    })
  })

  it('(g) primer carácter visible: para k = 1..40 cae en la columna k', async () => {
    const t = await setup(book)
    try {
      const starts = blockStarts(book)
      const pitch = measurePitch(t.columns)!
      for (let k = 1; k <= 40; k++) {
        t.component.goTo(k, 'instant')
        const i = firstVisibleCharIndex(t.viewport, t.columns)!
        expect(columnOfCharIndex(i, starts, t.viewport, t.columns, pitch), `k = ${k}`).toBe(k)
      }
    } finally {
      await t.cleanup()
    }
  })

  it('(h) ancla sobre un blanco U+00A0 lejano: monta en él y la columna visible es la suya (> 0)', async () => {
    const starts = blockStarts(book)
    const b = book.blocks.findIndex((x, i) => i > 300 && isBlank(x))
    expect(b).toBeGreaterThan(0)
    const idx = starts[b]
    const t = await setup({ ...book, initial_char_index: idx })
    try {
      const pitch = measurePitch(t.columns)!
      const esperada = columnOfCharIndex(idx, starts, t.viewport, t.columns, pitch)
      expect(esperada).toBeGreaterThan(0)
      expect(t.component.anchor()).toBe(idx)
      expect(Math.round(t.viewport.scrollLeft / pitch)).toBe(esperada)
    } finally {
      await t.cleanup()
    }
  })

  it('(i) búsqueda: rangeOf reproduce el texto en un párrafo con sangría y en un título; show lleva una lejana al centro', async () => {
    const t = await setup(book)
    try {
      const starts = blockStarts(book)
      const flat = flatText(book)
      const pi = geo(t).ps.findIndex((p, i) => i > 100 && p.classList.contains('indent'))
      const hi = book.blocks.findIndex((x, i) => i > 100 && x.type === 'Heading')
      expect(pi).toBeGreaterThan(0)
      for (const i of [pi, hi]) {
        const needle = runsOf(book.blocks[i])[0].text.slice(0, 12)
        const m = findAll(flat, needle).find((x) => x.start >= starts[i])!
        expect(flat.slice(m.start, m.end)).toBe(needle)
        expect(rangeOf(t.columns, starts, m)!.toString().split(SHY).join('')).toBe(needle)
      }

      const far = findAll(flat, 'Párrafo 301.')[0]!
      const pitch = geo(t).pitch
      const cs = getComputedStyle(t.columns)
      const vis = Math.floor((t.viewport.clientWidth + parseFloat(cs.columnGap)) / pitch)
      t.component.show(far, 'instant')
      await settle()
      const left = t.viewport.getBoundingClientRect().left + parseFloat(cs.paddingLeft)
      const col = Math.floor((charLeft(t.columns, far.start) - left + 1) / pitch)
      expect(col).toBe(Math.floor((vis - 1) / 2))
    } finally {
      CSS.highlights.delete('find')
      await t.cleanup()
    }
  })

  it('(j) paso = column_width × (1 + 2 × inset) con título centrado como primer bloque y capítulos', async () => {
    expect(book.blocks[0].type).toBe('Heading')
    expect(book.chapters.length).toBeGreaterThan(0)
    const t = await setup(book)
    try {
      const esperado = defaults.column_width * (1 + 2 * defaults.relative_inset * 0.01)
      expect(Math.abs(measurePitch(t.columns)! - esperado)).toBeLessThan(0.5)
    } finally {
      await t.cleanup()
    }
  })
})
