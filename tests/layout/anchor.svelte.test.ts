import { beforeAll, describe, it, expect } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import type { Doc } from '../../src/lib/ipc/types/Doc'
import type { Prefs } from '../../src/lib/ipc/types/Prefs'
import type { Match } from '../../src/lib/find/search'
import Columns from '../../src/components/Columns.svelte'
import { measurePitch } from '../../src/lib/nav/columnScroll'
import { blockStarts, flatText } from '../../src/lib/layout/textIndex'
import { findAll } from '../../src/lib/find/search'
import { rangeOf } from '../../src/lib/find/highlight'
import { SHY } from '../../src/lib/layout/hyphenate'
import { columnOfCharIndex, firstVisibleCharIndex } from '../../src/lib/layout/anchor'
import { defaults, fontReady, longDoc } from './defaults'

beforeAll(() => fontReady)

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

function charLeft(columns: HTMLElement, index: number): number {
  const ps = Array.from(columns.querySelectorAll<HTMLElement>('p[data-char-start]'))
  const p = ps.filter((el) => Number(el.dataset.charStart) <= index).pop()!
  const text = p.firstChild!
  // Posición en el DOM: el índice no cuenta los guiones blandos de la partición.
  const s = text.textContent!
  let offset = 0
  for (let c = index - Number(p.dataset.charStart); c > 0 || s[offset] === SHY; offset++) if (s[offset] !== SHY) c--
  const range = document.createRange()
  range.setStart(text, offset)
  range.setEnd(text, offset + 1)
  // Tras una palabra partida, el primer rectángulo es el guion de la línea anterior.
  const rects = range.getClientRects()
  return rects[rects.length - 1].left
}

async function setup(doc: Doc, onanchor?: (i: number) => void) {
  const host = document.createElement('div')
  host.style.cssText = 'width: 1000px; height: 700px;'
  document.body.appendChild(host)
  const props = $state<{ doc: Doc; prefs: Prefs; scale: number; fullscreen?: boolean; onanchor?: (i: number) => void }>({
    doc,
    prefs: { ...defaults },
    scale: 1,
    fullscreen: false,
    onanchor,
  })
  const component = mount(Columns, { target: host, props }) as unknown as Api & Record<string, unknown>
  flushSync()
  await frames()
  const viewport = host.querySelector<HTMLElement>('.viewport')!
  const columns = host.querySelector<HTMLElement>('.columns')!
  const cleanup = async () => {
    await unmount(component)
    host.remove()
  }
  return { host, props, component, viewport, columns, cleanup }
}

async function settle() {
  flushSync()
  await frames()
}

describe('ancla (P5)', () => {
  const changes: [string, (p: { prefs: Prefs; scale: number }, host: HTMLElement) => void][] = [
    ['font_size', (p) => (p.prefs.font_size = 18)],
    ['column_width', (p) => (p.prefs.column_width = 300)],
    ['line_height', (p) => (p.prefs.line_height = 1.6)],
    ['relative_inset', (p) => (p.prefs.relative_inset = 10)],
    ['vertical_inset', (p) => (p.prefs.vertical_inset = 60)],
    ['scale', (p) => (p.scale = 1.331)],
    ['ancho del host', (_, host) => (host.style.width = '1400px')],
  ]

  it.each(changes)('tras cambiar %s el ancla queda en la primera columna visible', async (_, change) => {
    const t = await setup(longDoc(400))
    try {
      t.component.goTo(37, 'instant')
      await scrollend(t.viewport)
      const anchor = t.component.anchor()
      expect(anchor).toBeGreaterThan(0)

      change(t.props, t.host)
      await settle()

      const pitch = measurePitch(t.columns)!
      const left = t.viewport.getBoundingClientRect().left
      const x = charLeft(t.columns, anchor)
      expect(x).toBeGreaterThanOrEqual(left - 0.5)
      expect(x).toBeLessThan(left + pitch)
      expect(t.component.anchor()).toBe(anchor)
    } finally {
      await t.cleanup()
    }
  })

  it('sin deriva: 20 cambios alternos de font_size dejan el ancla igual', async () => {
    const t = await setup(longDoc(400))
    try {
      t.component.goTo(37, 'instant')
      await scrollend(t.viewport)
      const anchor = t.component.anchor()
      for (let i = 0; i < 20; i++) {
        t.props.prefs.font_size = i % 2 === 0 ? 18 : 14
        await settle()
      }
      expect(t.component.anchor()).toBe(anchor)
    } finally {
      await t.cleanup()
    }
  })

  it('el primer carácter visible de cada columna está en esa columna (también tras palabra partida)', async () => {
    const t = await setup(longDoc(400))
    try {
      const pitch = measurePitch(t.columns)!
      const starts = blockStarts(t.props.doc)
      for (let k = 1; k <= 40; k++) {
        t.component.goTo(k, 'instant')
        const i = firstVisibleCharIndex(t.viewport, t.columns)!
        expect(columnOfCharIndex(i, starts, t.viewport, t.columns, pitch), `k = ${k}`).toBe(k)
      }
    } finally {
      await t.cleanup()
    }
  })

  it('500 saltos sin error acumulado', async () => {
    const t = await setup(longDoc(2600))
    try {
      const pitch = measurePitch(t.columns)!
      expect(t.viewport.scrollWidth / pitch).toBeGreaterThanOrEqual(501)
      const cs = getComputedStyle(t.columns)
      const padLeft = parseFloat(cs.paddingLeft)
      const y = t.viewport.getBoundingClientRect().top + parseFloat(cs.paddingTop) + parseFloat(cs.lineHeight) / 2

      for (let k = 1; k <= 500; k++) {
        t.component.goTo(k, 'instant')
        const left = t.viewport.getBoundingClientRect().left + padLeft
        const pos = document.caretPositionFromPoint(left + 0.5, y)!
        // El caret puede quedar antes del guion blando de la línea anterior: misma posición de índice.
        let offset = pos.offset
        while (pos.offsetNode.textContent![offset] === SHY) offset++
        const range = document.createRange()
        range.setStart(pos.offsetNode, offset)
        range.setEnd(pos.offsetNode, offset + 1)
        // Tras una palabra partida, el primer rectángulo es el guion de la línea anterior.
        const rects = range.getClientRects()
        expect(Math.abs(rects[rects.length - 1].left - left), `k = ${k}`).toBeLessThan(1)
      }
    } finally {
      await t.cleanup()
    }
  })
})

describe('posición inicial (fase 3)', () => {
  async function mountAtMiddle() {
    const base = longDoc(400)
    const idx = blockStarts(base)[200] + 10
    const calls: number[] = []
    const t = await setup({ ...base, initial_char_index: idx }, (i) => calls.push(i))
    await scrollend(t.viewport)
    await settle()
    return { t, idx, calls }
  }

  it('monta en el carácter inicial sin interacción y sin llamar a onanchor', async () => {
    const { t, idx, calls } = await mountAtMiddle()
    try {
      const pitch = measurePitch(t.columns)!
      const left = t.viewport.getBoundingClientRect().left
      const x = charLeft(t.columns, idx)
      expect(x).toBeGreaterThanOrEqual(left - 0.5)
      expect(x).toBeLessThan(left + pitch)
      expect(t.component.anchor()).toBe(idx)
      expect(calls).toHaveLength(0)
    } finally {
      await t.cleanup()
    }
  })

  it('un goTo posterior llama a onanchor una vez con la nueva ancla', async () => {
    const { t, idx, calls } = await mountAtMiddle()
    try {
      const pitch = measurePitch(t.columns)!
      const column = Math.round(t.viewport.scrollLeft / pitch)
      t.component.goTo(column + 3, 'instant')
      await scrollend(t.viewport)
      expect(calls).toHaveLength(1)
      expect(calls[0]).toBe(t.component.anchor())
      expect(calls[0]).not.toBe(idx)
    } finally {
      await t.cleanup()
    }
  })
})

describe('búsqueda (fase 4)', () => {
  function docTwoRuns() {
    return {
      title: 'runs',
      initial_char_index: 0,
      composition: 'Text' as const,
      chapters: [] as number[],
      blocks: [{ type: 'Paragraph' as const, runs: [{ text: 'abcd' }, { text: 'efgh' }] }],
    }
  }

  it('rangeOf reconstruye el texto de la coincidencia', async () => {
    const t = await setup(longDoc(400))
    try {
      const m = findAll(flatText(t.props.doc), 'Párrafo 200.')[0]
      expect(m).toBeDefined()
      const starts = blockStarts(t.props.doc)
      const r = rangeOf(t.columns, starts, m!)
      expect(r).not.toBeNull()
      // El DOM lleva los guiones blandos de la partición; el índice no.
      expect(r!.toString().split(SHY).join('')).toBe(flatText(t.props.doc).slice(m!.start, m!.end))
    } finally {
      CSS.highlights.delete('find')
      await t.cleanup()
    }
  })

  it('rangeOf con una coincidencia partida entre dos runs', async () => {
    const t = await setup(docTwoRuns())
    try {
      const starts = blockStarts(t.props.doc)
      const m = { start: 2, end: 6 }
      const r = rangeOf(t.columns, starts, m)
      expect(r!.toString()).toBe('cdef')
      const m2 = { start: 5, end: 8 }
      const r2 = rangeOf(t.columns, starts, m2)
      expect(r2!.toString()).toBe('fgh')
    } finally {
      CSS.highlights.delete('find')
      await t.cleanup()
    }
  })

  it('show de una coincidencia lejana la deja en la columna central', async () => {
    const t = await setup(longDoc(400))
    try {
      const m = findAll(flatText(t.props.doc), 'Párrafo 300.')[0]!
      const pitch = measurePitch(t.columns)!
      const cs = getComputedStyle(t.columns)
      const gap = parseFloat(cs.columnGap)
      const vis = Math.floor((t.viewport.clientWidth + gap) / pitch)
      await t.component.show(m, 'instant')
      await settle()
      const start = charLeft(t.columns, m.start)
      const col = Math.floor((start - t.viewport.getBoundingClientRect().left - parseFloat(cs.paddingLeft) + 1) / pitch)
      expect(col).toBe(Math.floor((vis - 1) / 2))
      // −1: scrollLeft se redondea a píxel entero y el paso es fraccionario.
      expect(start).toBeGreaterThanOrEqual(t.viewport.getBoundingClientRect().left + parseFloat(cs.paddingLeft) - 1)
      expect(start).toBeLessThan(t.viewport.getBoundingClientRect().left + t.viewport.clientWidth - parseFloat(cs.paddingRight))
    } finally {
      CSS.highlights.delete('find')
      await t.cleanup()
    }
  })

  it('show de una coincidencia ya visible (última columna) no mueve el viewport', async () => {
    const t = await setup(longDoc(400))
    try {
      t.component.goTo(10, 'instant')
      await scrollend(t.viewport)
      const pitch = measurePitch(t.columns)!
      const cs = getComputedStyle(t.columns)
      const vis = Math.floor((t.viewport.clientWidth + parseFloat(cs.columnGap)) / pitch)
      const left = t.viewport.getBoundingClientRect().left + parseFloat(cs.paddingLeft)
      const column = (m: Match) => Math.floor((charLeft(t.columns, m.start) - left + 1) / pitch)
      const m = findAll(flatText(t.props.doc), 'Párrafo ').find((x) => column(x) === vis - 1)
      expect(vis).toBeGreaterThan(1)
      expect(m).toBeDefined()
      const before = t.viewport.scrollLeft
      t.component.show(m!, 'instant')
      await settle()
      expect(t.viewport.scrollLeft).toBe(before)
    } finally {
      CSS.highlights.delete('find')
      await t.cleanup()
    }
  })

  it('CSS.highlights se gestiona correctamente', async () => {
    const t = await setup(longDoc(400))
    try {
      const m = findAll(flatText(t.props.doc), 'Párrafo 0.')[0]!
      expect(CSS.highlights.get('find')).toBeUndefined()
      await t.component.show(m, 'instant')
      await settle()
      expect(CSS.highlights.get('find')!.size).toBe(1)
      await t.component.show(null, 'instant')
      expect(CSS.highlights.has('find')).toBe(false)
      await t.component.show(m, 'instant')
      await settle()
      expect(CSS.highlights.get('find')!.size).toBe(1)
    } finally {
      await t.cleanup()
    }
    expect(CSS.highlights.has('find')).toBe(false)
  })
})

describe('pantalla completa (P5)', () => {
  it('alternar pantalla completa conserva el ancla', async () => {
    const t = await setup(longDoc(400))
    try {
      t.component.goTo(5, 'instant')
      await scrollend(t.viewport)
      const anchor = t.component.anchor()
      expect(anchor).toBeGreaterThan(0)

      for (const fullscreen of [true, false]) {
        t.props.fullscreen = fullscreen
        await settle()
        const pitch = measurePitch(t.columns)!
        const left = t.viewport.getBoundingClientRect().left
        const x = charLeft(t.columns, anchor)
        expect(x, `fullscreen = ${fullscreen}`).toBeGreaterThanOrEqual(left - 0.5)
        expect(x, `fullscreen = ${fullscreen}`).toBeLessThan(left + pitch)
        expect(t.component.anchor()).toBe(anchor)
      }
    } finally {
      await t.cleanup()
    }
  })
})
