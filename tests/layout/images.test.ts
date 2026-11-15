import { beforeAll, describe, it, expect } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import type { Block } from '../../src/lib/ipc/types/Block'
import type { Doc } from '../../src/lib/ipc/types/Doc'
import Columns from '../../src/components/Columns.svelte'
import { measurePitch } from '../../src/lib/nav/columnScroll'
import { blockAt, blockStarts } from '../../src/lib/layout/textIndex'
import { columnOfCharIndex, firstVisibleCharIndex } from '../../src/lib/layout/anchor'
import { SHY } from '../../src/lib/layout/hyphenate'
import { defaults, fontReady, longDoc } from './defaults'

beforeAll(() => fontReady)

// Como miso-img: la URL la da el stub y la imagen llega de forma asíncrona. Sin registro, no decodifica.
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

const para = (text: string): Block => ({ type: 'Paragraph', runs: [{ text }] })

async function withImages(paragraphs: Block[], spec: (n: number) => [number, number, number] | undefined): Promise<Block[]> {
  const out: Block[] = []
  for (let n = 0; n < paragraphs.length; n++) {
    out.push(paragraphs[n])
    const s = spec(n)
    if (s) out.push(await image(s[0], s[1], s[2]))
  }
  return out
}

const frames = (n = 3) =>
  new Promise<void>((resolve) => {
    const step = (left: number) => (left === 0 ? resolve() : requestAnimationFrame(() => step(left - 1)))
    step(n)
  })

async function mountBook(doc: Doc) {
  const host = document.createElement('div')
  host.style.cssText = 'width: 1000px; height: 700px;'
  document.body.appendChild(host)
  const component = mount(Columns, { target: host, props: { doc, prefs: defaults, scale: 1 } }) as unknown as {
    goTo(k: number, behavior?: ScrollBehavior): void
    anchor(): number
  } & Record<string, unknown>
  flushSync()
  await Promise.all(Array.from(host.querySelectorAll('img'), (img) => img.decode().catch(() => undefined)))
  flushSync()
  await frames()
  return {
    host,
    component,
    viewport: host.querySelector<HTMLElement>('.viewport')!,
    columns: host.querySelector<HTMLElement>('.columns')!,
    cleanup: async () => {
      await unmount(component)
      host.remove()
    },
  }
}

function charLeft(columns: HTMLElement, index: number): number {
  const p = Array.from(columns.querySelectorAll<HTMLElement>('p[data-char-start]'))
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

describe('imágenes (P5)', () => {
  it('una imagen alta cabe en la altura de columna y no se parte', async () => {
    const doc: Doc = {
      title: 'alta',
      initial_char_index: 0,
      composition: 'Text',
      chapters: [],
      blocks: [para('Antes.'), await image(101, 200, 3000), para('Después.')],
    }
    const t = await mountBook(doc)
    try {
      const p = t.columns.querySelectorAll<HTMLElement>('p[data-char-start]')[1]
      const img = p.querySelector('img')!
      const alto = t.viewport.clientHeight - 2 * defaults.vertical_inset
      expect(p.getClientRects()).toHaveLength(1)
      expect(img.getBoundingClientRect().height).toBeGreaterThan(0)
      expect(img.getBoundingClientRect().height).toBeLessThanOrEqual(alto + 0.5)
    } finally {
      await t.cleanup()
    }
  })

  it('una imagen ancha cabe en el ancho de columna', async () => {
    const doc: Doc = {
      title: 'ancha',
      initial_char_index: 0,
      composition: 'Text',
      chapters: [],
      blocks: [para('Antes.'), await image(102, 3000, 100), para('Después.')],
    }
    const t = await mountBook(doc)
    try {
      const p = t.columns.querySelectorAll<HTMLElement>('p[data-char-start]')[1]
      const img = p.querySelector('img')!
      expect(img.getBoundingClientRect().width).toBeGreaterThan(0)
      expect(img.getBoundingClientRect().width).toBeLessThanOrEqual(p.getBoundingClientRect().width + 0.5)
    } finally {
      await t.cleanup()
    }
  })

  it('initial_char_index tras imágenes anteriores queda en la primera columna visible', async () => {
    const blocks = await withImages(longDoc(120).blocks, (n) => (n % 3 === 2 ? [1000 + n, 2400, 1800] : undefined))
    const starts = blockStarts({ title: 'libro', initial_char_index: 0, composition: 'Text', chapters: [], blocks })
    const k = blocks.findIndex((b, i) => i >= 90 && b.type === 'Paragraph')
    const idx = starts[k] + 10
    const t = await mountBook({ title: 'libro', initial_char_index: idx, composition: 'Text', chapters: [], blocks })
    try {
      const pitch = measurePitch(t.columns)!
      const left = t.viewport.getBoundingClientRect().left
      const x = charLeft(t.columns, idx)
      expect(x).toBeGreaterThanOrEqual(left - 0.5)
      expect(x).toBeLessThan(left + pitch)
      expect(t.component.anchor()).toBe(idx)
    } finally {
      await t.cleanup()
    }
  })

  it('una columna que empieza con una imagen tiene como índice el inicio del bloque imagen', async () => {
    const blocks = await withImages(longDoc(150).blocks, (n) => (n % 2 === 1 ? [2000 + n, 240, 180] : undefined))
    const doc: Doc = { title: 'columnas', initial_char_index: 0, composition: 'Text', chapters: [], blocks }
    const starts = blockStarts(doc)
    const t = await mountBook(doc)
    try {
      const pitch = measurePitch(t.columns)!
      let halladas = 0
      for (let k = 1; k <= 40; k++) {
        t.component.goTo(k, 'instant')
        const i = firstVisibleCharIndex(t.viewport, t.columns)!
        const b = blockAt(starts, i)
        if (blocks[b].type !== 'Image') continue
        halladas++
        expect(i, `k = ${k}`).toBe(starts[b])
        expect(columnOfCharIndex(i, starts, t.viewport, t.columns, pitch), `k = ${k}`).toBe(k)
      }
      expect(halladas).toBeGreaterThan(0)
    } finally {
      await t.cleanup()
    }
  })

  it('measurePitch es el del paso de columna con una imagen como primer bloque', async () => {
    const doc: Doc = {
      title: 'primera',
      initial_char_index: 0,
      composition: 'Text',
      chapters: [],
      blocks: [await image(3000, 240, 180), ...longDoc(60).blocks],
    }
    const t = await mountBook(doc)
    try {
      const esperado = defaults.column_width * (1 + 2 * defaults.relative_inset * 0.01)
      expect(Math.abs(measurePitch(t.columns)! - esperado)).toBeLessThan(0.5)
    } finally {
      await t.cleanup()
    }
  })

  it('una imagen que no decodifica queda oculta y el ancla se conserva', async () => {
    const blocks: Block[] = []
    longDoc(120).blocks.forEach((b, n) => {
      blocks.push(b)
      if (n === 5) blocks.push({ type: 'Image', id: 4000 })
    })
    const flat: Doc = { title: 'rota', initial_char_index: 0, composition: 'Text', chapters: [], blocks }
    const starts = blockStarts(flat)
    const k = blocks.findIndex((b, i) => i > 60 && b.type === 'Paragraph')
    const idx = starts[k] + 10
    const t = await mountBook({ ...flat, initial_char_index: idx })
    try {
      const img = t.columns.querySelector<HTMLImageElement>('img')!
      expect(img.hidden).toBe(true)
      expect(img.getClientRects()).toHaveLength(0)
      expect(t.component.anchor()).toBe(idx)
      const pitch = measurePitch(t.columns)!
      const left = t.viewport.getBoundingClientRect().left
      const x = charLeft(t.columns, idx)
      expect(x).toBeGreaterThanOrEqual(left - 0.5)
      expect(x).toBeLessThan(left + pitch)
    } finally {
      await t.cleanup()
    }
  })
})
