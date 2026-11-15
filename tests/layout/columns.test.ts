import { beforeAll, describe, it, expect } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import unicodeJson from '../fixtures/unicode.doc.json'
import type { Doc } from '../../src/lib/ipc/types/Doc'
import Columns from '../../src/components/Columns.svelte'
import { fitViewport, fullscreenColumnWidth } from '../../src/lib/layout/columnWidth'
import { blockStarts, flatLength, runsOf } from '../../src/lib/layout/textIndex'
import { measurePitch } from '../../src/lib/nav/columnScroll'
import { columnOfCharIndex, firstVisibleCharIndex } from '../../src/lib/layout/anchor'
import { rangeOf } from '../../src/lib/find/highlight'
import { SHY } from '../../src/lib/layout/hyphenate'
import { defaults, fontReady, longDoc as makeLongDoc, textRects } from './defaults'

beforeAll(() => fontReady)

it('Literata se carga desde los recursos de la app', async () => {
  expect((await fontReady).length).toBeGreaterThan(0)
})

const doc = unicodeJson as Doc
const K = 25
const longDoc: Doc = makeLongDoc(400)

describe('Columns (D9: el DOM no añade texto entre bloques)', () => {
  it('un <p> por bloque con el texto de sus runs, data-char-start = blockStarts, último start + longitud = K', async () => {
    const host = document.createElement('div')
    host.style.cssText = 'width: 1000px; height: 700px;'
    document.body.appendChild(host)
    let component: Record<string, unknown> | undefined

    try {
      component = mount(Columns, { target: host, props: { doc, prefs: defaults, scale: 1 } })

      const container = host.querySelector('p')!.parentElement!
      expect(
        Array.from(container.childNodes)
          .filter((node) => node.nodeType === Node.TEXT_NODE && node.textContent !== '')
          .map((node) => JSON.stringify(node.textContent)),
      ).toEqual([])

      const paragraphs = Array.from(host.querySelectorAll('p'))
      expect(paragraphs.map((p) => p.textContent)).toEqual(
        doc.blocks.map((block) => runsOf(block).map((run) => run.text).join('')),
      )
      expect(paragraphs.map((p) => Number(p.dataset.charStart))).toEqual(blockStarts(doc))

      const last = paragraphs[paragraphs.length - 1]
      expect(Number(last.dataset.charStart) + last.textContent!.length).toBe(K)
      expect(flatLength(doc)).toBe(K)
    } finally {
      if (component) await unmount(component)
      host.remove()
    }
  })
})

type ColumnsApi = { goTo(k: number, behavior?: ScrollBehavior): void; visibleColumns(): number }

function mountHost(width = 1000): HTMLDivElement {
  const host = document.createElement('div')
  host.style.cssText = `width: ${width}px; height: 700px;`
  document.body.appendChild(host)
  return host
}

describe('navegación por columnas (G6, G7)', () => {
  it('measurePitch = diferencia de left entre dos columnas consecutivas (±0.5 px)', async () => {
    const host = mountHost()
    let component: Record<string, unknown> | undefined

    try {
      component = mount(Columns, { target: host, props: { doc: longDoc, prefs: defaults, scale: 1 } })
      flushSync()

      const columns = host.querySelector<HTMLElement>('.columns')!
      const lefts = textRects(columns).map((rect) => rect.left)
      const col0 = Math.min(...lefts)
      const col1 = Math.min(...lefts.filter((left) => left - col0 > 1))
      const pitch = measurePitch(columns)

      expect(pitch).not.toBeNull()
      expect(Math.abs(pitch! - (col1 - col0))).toBeLessThan(0.5)
    } finally {
      if (component) await unmount(component)
      host.remove()
    }
  })

  it('goTo(k, instant) para k = 1..20: la columna k queda pegada al padding izquierdo', async () => {
    const host = mountHost()
    let component: Record<string, unknown> | undefined

    try {
      component = mount(Columns, { target: host, props: { doc: longDoc, prefs: defaults, scale: 1 } })
      flushSync()

      const api = component as ColumnsApi
      const viewport = host.querySelector<HTMLElement>('.viewport')!
      const columns = host.querySelector<HTMLElement>('.columns')!
      const padLeft = parseFloat(getComputedStyle(columns).paddingLeft)

      for (let col = 1; col <= 20; col++) {
        api.goTo(col, 'instant')
        const vp = viewport.getBoundingClientRect()
        const visibleLefts = textRects(columns)
          .filter((rect) => rect.right > vp.left && rect.left < vp.right)
          .map((rect) => rect.left)
        expect(Math.abs(Math.min(...visibleLefts) - (vp.left + padLeft))).toBeLessThan(1)
      }
    } finally {
      if (component) await unmount(component)
      host.remove()
    }
  })
})

it('una línea en blanco mide media línea', async () => {
  const host = mountHost()
  const blank: Doc = {
    title: 'b',
    initial_char_index: 0,
    composition: 'Text',
    chapters: [],
    blocks: ['Uno.', '', 'Dos.'].map((text) => ({ type: 'Paragraph' as const, runs: [{ text }] })),
  }
  const component = mount(Columns, { target: host, props: { doc: blank, prefs: defaults, scale: 1 } })
  try {
    flushSync()
    const [, gap] = host.querySelectorAll('p')
    const line = parseFloat(getComputedStyle(host.querySelector('.columns')!).lineHeight)
    expect(Math.abs(gap.getBoundingClientRect().height - line / 2)).toBeLessThan(0.5)
  } finally {
    await unmount(component)
    host.remove()
  }
})

function richDoc(): Doc {
  const body = makeLongDoc(120).blocks
  const blocks: Doc['blocks'] = [
    { type: 'Quote', runs: [{ text: 'Una cita que abre la primera columna del documento.' }] },
    { type: 'ListItem', depth: 0, runs: [{ text: '• ' }, { text: 'Primer elemento de la lista.' }] },
    { type: 'Heading', level: 2, runs: [{ text: 'Un título de sección' }] },
    ...body.map((block, i) =>
      i % 4 === 0
        ? {
            type: 'Paragraph' as const,
            runs: [
              { text: 'Antes del estilo ' },
              { text: 'negrita en medio', bold: true },
              { text: ` y después del estilo. ${block.runs[0].text}` },
            ],
          }
        : block,
    ),
  ]
  return { title: 'rico', initial_char_index: 0, composition: 'Text' as const, chapters: [], blocks }
}

const rico = richDoc()
const plano: Doc = {
  ...rico,
  blocks: rico.blocks.map((block) => ({ type: 'Paragraph' as const, runs: [{ text: runsOf(block).map((r) => r.text).join('') }] })),
}

describe('documento rico (fase 5, P2)', () => {
  it('cada <p> conserva su texto: sin guiones blandos, textContent = runs unidos', async () => {
    const host = mountHost()
    const component = mount(Columns, { target: host, props: { doc: rico, prefs: defaults, scale: 1 } })
    try {
      flushSync()
      const paragraphs = Array.from(host.querySelectorAll('p'))
      expect(paragraphs.map((p) => p.textContent!.split(SHY).join(''))).toEqual(
        rico.blocks.map((block) => runsOf(block).map((run) => run.text).join('')),
      )
    } finally {
      await unmount(component)
      host.remove()
    }
  })

  it('el pitch con cita, lista y título es el mismo que sin sangrías', async () => {
    const pitchOf = async (doc: Doc) => {
      const host = mountHost()
      const component = mount(Columns, { target: host, props: { doc, prefs: defaults, scale: 1 } })
      try {
        flushSync()
        return measurePitch(host.querySelector<HTMLElement>('.columns')!)!
      } finally {
        await unmount(component)
        host.remove()
      }
    }
    const conTipos = await pitchOf(rico)
    const sinTipos = await pitchOf(plano)
    expect(Math.abs(conTipos - sinTipos)).toBeLessThan(1)
    expect(Math.abs(conTipos - defaults.column_width * (1 + 2 * defaults.relative_inset * 0.01))).toBeLessThan(0.5)
  })

  it('firstVisibleCharIndex da índices del modelo en cada columna (también dentro de un <span>)', async () => {
    const host = mountHost()
    const component = mount(Columns, { target: host, props: { doc: rico, prefs: defaults, scale: 1 } }) as unknown as ColumnsApi & Record<string, unknown>
    try {
      flushSync()
      const viewport = host.querySelector<HTMLElement>('.viewport')!
      const columns = host.querySelector<HTMLElement>('.columns')!
      const starts = blockStarts(rico)
      const pitch = measurePitch(columns)!
      for (let k = 0; k <= 30; k++) {
        component.goTo(k, 'instant')
        const i = firstVisibleCharIndex(viewport, columns)!
        expect(columnOfCharIndex(i, starts, viewport, columns, pitch), `k = ${k}`).toBe(k)
      }
    } finally {
      await unmount(component)
      host.remove()
    }
  })

  it('rangeOf cruza de un <span> al texto siguiente con índices del modelo', async () => {
    const host = mountHost()
    const component = mount(Columns, { target: host, props: { doc: rico, prefs: defaults, scale: 1 } })
    try {
      flushSync()
      const columns = host.querySelector<HTMLElement>('.columns')!
      const starts = blockStarts(rico)
      const block = runsOf(rico.blocks[3]).map((run) => run.text).join('')
      const at = block.indexOf('medio y')
      const range = rangeOf(columns, starts, { start: starts[3] + at, end: starts[3] + at + 'medio y'.length })
      expect(range!.toString().split(SHY).join('')).toBe('medio y')
    } finally {
      await unmount(component)
      host.remove()
    }
  })

  it('un título no hereda un interlineado menor que su letra', async () => {
    const host = mountHost()
    const titulo = 'Un título bastante largo que ocupa varias líneas dentro de una columna estrecha del lector. '.repeat(2)
    const doc: Doc = {
      title: 't',
      initial_char_index: 0,
      composition: 'Text',
      chapters: [],
      blocks: [
        { type: 'Heading', level: 1, runs: [{ text: titulo }] },
        { type: 'Paragraph', runs: [{ text: 'Párrafo siguiente.' }] },
      ],
    }
    // Con el interlineado mínimo, el heredado del párrafo (≈ 20 px) queda por debajo de la letra del título (21 px).
    const component = mount(Columns, { target: host, props: { doc, prefs: { ...defaults, line_height: 1 }, scale: 1 } })
    try {
      flushSync()
      const [h1, next] = Array.from(host.querySelectorAll<HTMLElement>('p'))
      expect(h1.classList.contains('h1')).toBe(true)
      const size = parseFloat(getComputedStyle(h1).fontSize)
      expect(size).toBeGreaterThan(parseFloat(getComputedStyle(next).fontSize))
      expect(parseFloat(getComputedStyle(h1).lineHeight)).toBeGreaterThanOrEqual(size)
      expect(h1.getBoundingClientRect().bottom).toBeLessThanOrEqual(next.getBoundingClientRect().top + 0.5)
    } finally {
      await unmount(component)
      host.remove()
    }
  })
})

describe('viewport exacto (P4)', () => {
  it.each([360, 300])('column_width %d: pitch = ancho + 2 × inset y viewport = fitViewport', async (column_width) => {
    const host = mountHost()
    let component: Record<string, unknown> | undefined

    try {
      const prefs = { ...defaults, column_width }
      component = mount(Columns, { target: host, props: { doc: longDoc, prefs, scale: 1 } })
      flushSync()

      const expected = column_width + 2 * prefs.relative_inset * 0.01 * column_width
      const viewport = host.querySelector<HTMLElement>('.viewport')!
      const pitch = measurePitch(host.querySelector<HTMLElement>('.columns')!)
      expect(Math.abs(pitch! - expected)).toBeLessThan(0.5)
      expect(Math.abs(viewport.getBoundingClientRect().width - fitViewport(1000, expected))).toBeLessThan(0.5)
    } finally {
      if (component) await unmount(component)
      host.remove()
    }
  })
})

describe('pantalla completa (P5, regla √2)', () => {
  it('el viewport llena el marco y caben exactamente las columnas de la regla', async () => {
    const host = mountHost(1500)
    let component: Record<string, unknown> | undefined

    try {
      component = mount(Columns, { target: host, props: { doc: longDoc, prefs: defaults, scale: 1, fullscreen: true } })
      flushSync()

      const cw = fullscreenColumnWidth(1500, defaults.column_width, defaults.relative_inset)
      const pitch = cw * (1 + 2 * defaults.relative_inset * 0.01)
      const cols = Math.round(1500 / pitch)
      const viewport = host.querySelector<HTMLElement>('.viewport')!

      expect(cols).toBe(4)
      expect(Math.abs(viewport.getBoundingClientRect().width - host.clientWidth)).toBeLessThan(1)
      expect(Math.abs(measurePitch(host.querySelector<HTMLElement>('.columns')!)! - pitch)).toBeLessThan(0.5)
      expect((component as ColumnsApi).visibleColumns()).toBe(cols)
    } finally {
      if (component) await unmount(component)
      host.remove()
    }
  })
})
