<script lang="ts">
  import { onMount } from 'svelte'
  import type { Block } from '../lib/ipc/types/Block'
  import type { Doc } from '../lib/ipc/types/Doc'
  import type { Run } from '../lib/ipc/types/Run'
  import type { Prefs } from '../lib/ipc/types/Prefs'
  import { fitViewport, fullscreenColumnWidth } from '../lib/layout/columnWidth'
  import { imageSrc } from '../lib/ipc/commands'
  import { blockStarts, runsOf } from '../lib/layout/textIndex'
  import { bookClasses } from '../lib/layout/composition'
  import { SHY, detectLang, hyphenate } from '../lib/layout/hyphenate'
  import { createColumnScroller } from '../lib/nav/columnScroll'
  import { scrollByWheel } from '../lib/nav/wheel'
  import { rangeOf, showMatch } from '../lib/find/highlight'
  import type { Match } from '../lib/find/search'

  let {
    doc,
    prefs,
    scale,
    fullscreen = false,
    onanchor,
  }: { doc: Doc; prefs: Prefs; scale: number; fullscreen?: boolean; onanchor?: (i: number) => void } = $props()
  const starts = $derived(blockStarts(doc))
  const book = $derived(bookClasses(doc))
  const lang = $derived(detectLang(doc))

  function kind(block: Block): string {
    if (block.type === 'Heading') return `h${block.level}`
    if (block.type === 'ListItem') return 'li'
    if (block.type === 'Quote') return 'quote'
    return ''
  }

  // Sin bloques de control dentro del <p> sin estilo: su primer hijo sigue siendo el texto.
  const styled = (run: Run) => !!(run.bold || run.italic || run.underline)

  function attrs(block: Block, i: number) {
    const blank = runsOf(block).every((r) => r.text === '')
    const classes = [kind(block), blank ? 'blank' : '', book[i]].join(' ').split(' ').filter(Boolean)
    return {
      'data-char-start': starts[i],
      class: [...new Set(classes)].join(' '),
      style: block.type === 'ListItem' ? `--depth: ${block.depth}` : undefined,
    }
  }

  // Lo copiado no lleva los guiones blandos de la partición.
  function oncopy(e: ClipboardEvent) {
    const text = getSelection()?.toString()
    if (!text || !e.clipboardData) return
    e.preventDefault()
    e.clipboardData.setData('text/plain', text.split(SHY).join(''))
  }

  const font = $derived(`"${prefs.font_family}", serif`)
  const size = $derived(prefs.font_size * scale)

  let frame: HTMLDivElement
  let viewport: HTMLDivElement
  let columns: HTMLDivElement
  let scroller: ReturnType<typeof createColumnScroller> | undefined

  // Imperativo: un $state intermedio dejaría correr el $effect antes de que el DOM tenga el ancho nuevo.
  // Equivalencia con AppKit (ARCHITECTURE §6): 5 px de lineFragmentPadding por lado sin cambiar el paso.
  function fit() {
    const cw = fullscreen
      ? fullscreenColumnWidth(frame.clientWidth, prefs.column_width, prefs.relative_inset)
      : prefs.column_width
    const inset = prefs.relative_inset * 0.01 * cw
    frame.style.setProperty('--vp-w', `${fitViewport(frame.clientWidth, cw + 2 * inset)}px`)
    frame.style.setProperty('--col-w', `${cw - 10}px`)
    frame.style.setProperty('--pad', `${inset + 5}px`)
    frame.style.setProperty('--gap', `${2 * inset + 10}px`)
    fitHeight()
  }

  function fitHeight() {
    frame.style.setProperty('--col-h', `${viewport.clientHeight - 2 * prefs.vertical_inset}px`)
  }

  onMount(() => {
    const s = createColumnScroller(viewport, columns, () => starts, doc.initial_char_index, (i) => onanchor?.(i))
    const frameResize = new ResizeObserver(fit)
    const resize = new ResizeObserver(() => {
      fitHeight()
      s.restore()
    })
    const onWheel = (e: WheelEvent) => scrollByWheel(e, viewport)
    scroller = s
    fit()
    s.restore()
    const images = Array.from(columns.querySelectorAll('img'))
    if (images.length > 0) {
      Promise.all(images.map((img) => img.decode().catch(() => (img.hidden = true)))).then(() => {
        if (scroller) {
          fit()
          scroller.restore()
        }
      })
    }
    frameResize.observe(frame)
    resize.observe(viewport)
    viewport.addEventListener('scrollend', s.sync)
    viewport.addEventListener('wheel', onWheel, { passive: false })
    return () => {
      frameResize.disconnect()
      resize.disconnect()
      viewport.removeEventListener('scrollend', s.sync)
      viewport.removeEventListener('wheel', onWheel)
      scroller = undefined
      showMatch(null)
    }
  })

  $effect(() => {
    void [font, size, prefs.column_width, prefs.relative_inset, prefs.vertical_inset, prefs.line_height, fullscreen]
    if (!scroller) return
    fit()
    scroller.restore()
  })

  export function go(delta: number) {
    scroller?.go(delta)
  }

  export function goTo(k: number, behavior?: ScrollBehavior) {
    scroller?.goTo(k, behavior)
  }

  export function first() {
    scroller?.first()
  }

  export function last() {
    scroller?.last()
  }

  export function visibleColumns(): number {
    return scroller?.visibleColumns() ?? 1
  }

  export function anchor(): number {
    return scroller?.anchor() ?? 0
  }

  export function show(m: Match | null, behavior?: ScrollBehavior): void {
    showMatch(m ? rangeOf(columns, starts, m) : null)
    if (m) scroller?.reveal(m.start, behavior)
  }
</script>

<div
  class="frame"
  bind:this={frame}
  style:--v-inset="{prefs.vertical_inset}px"
  style:--font={font}
  style:--size="{size}px"
  style:--lh={prefs.line_height}
  style:--fg={prefs.foreground}
>
  <div class="viewport" bind:this={viewport}>
    <div class="columns" class:book={doc.composition === 'Book'} lang={lang ?? navigator.language} {oncopy} bind:this={columns}>{#each doc.blocks as block, i}{#if block.type === 'Image'}<p data-char-start={starts[i]} class={['img', book[i]].filter(Boolean).join(' ')}><img src={imageSrc(block.id)} alt="" /></p>{:else if runsOf(block).some(styled)}<p {...attrs(block, i)}>{#each runsOf(block) as run}{#if styled(run)}<span class:b={run.bold} class:i={run.italic} class:u={run.underline}>{hyphenate(run.text, lang)}</span>{:else}{hyphenate(run.text, lang)}{/if}{/each}</p>{:else}<p {...attrs(block, i)}>{#each runsOf(block) as run}{hyphenate(run.text, lang)}{/each}</p>{/if}{/each}</div>
  </div>
</div>

<style>
  .frame {
    height: 100%;
  }

  /* Interlineado como Tofu: --lh × alto natural de la fuente (1lh = line-height: normal del padre). */
  .viewport {
    width: var(--vp-w);
    margin-inline: auto;
    height: 100%;
    overflow-x: auto;
    overflow-y: hidden;
    overflow-anchor: none;
    font-family: var(--font);
    font-size: var(--size);
    /* Más cuerpo que el 400 de Windows, cerca del trazo de macOS; las fuentes no variables caen a 400. */
    font-weight: 450;
    line-height: normal;
    color: var(--fg);
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--fg) 35%, transparent) transparent;
  }

  .columns {
    height: 100%;
    box-sizing: border-box;
    column-width: var(--col-w);
    column-gap: var(--gap);
    column-fill: auto;
    padding: var(--v-inset) var(--pad);
    line-height: calc(var(--lh) * 1lh);
    text-align: justify;
  }

  p {
    margin: 0;
  }

  /* Una línea en blanco separa párrafos: media línea, como el blanco entre párrafos de un libro. */
  p.blank {
    height: 0.5lh;
  }

  /* Composición de libro: la separación es margen (el salto de columna no forzado lo trunca). */
  .book p.sep {
    margin-block-start: 0.5lh;
  }

  .book p.indent {
    text-indent: 1.5em;
  }

  .book p.chapter {
    break-before: column;
    margin-block-start: 0;
  }

  .book p.blank {
    height: 0;
    line-height: 0;
  }

  /* Una imagen no se parte y cabe en una columna; el ancla la cuenta como un bloque de 0 unidades. */
  p.img {
    break-inside: avoid;
  }

  img {
    margin-inline: auto;
    max-width: 100%;
    max-height: var(--col-h);
  }

  /* Con display: block del autor, el atributo hidden de una imagen rota no ocultaría nada. */
  img:not([hidden]) {
    display: block;
  }

  /* Sangrías con padding, nunca margin: la caja de cada <p> empieza en el borde de su columna. */
  .h1,
  .h2,
  .h3,
  .h4,
  .h5,
  .h6 {
    font-weight: 700;
    line-height: calc(var(--lh) * 1.2);
    text-align: center;
    break-after: avoid;
  }

  .h1 {
    font-size: 1.5em;
  }

  .h2 {
    font-size: 1.3em;
  }

  .h3 {
    font-size: 1.15em;
  }

  .h4,
  .h5,
  .h6 {
    font-size: 1em;
  }

  .li {
    padding-inline-start: calc((min(var(--depth), 3) + 1) * 1.5em);
    text-indent: -1.5em;
  }

  .quote {
    padding-inline: 1.5em;
  }

  .b {
    font-weight: 700;
  }

  .i {
    font-style: italic;
  }

  .u {
    text-decoration: underline;
  }

  :global(::highlight(find)) {
    background-color: Highlight;
    color: HighlightText;
  }
</style>
