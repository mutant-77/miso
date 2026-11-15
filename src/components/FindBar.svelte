<script lang="ts">
  import { onMount } from 'svelte'
  import type { Match } from '../lib/find/search'
  import { findAll, firstAtOrAfter } from '../lib/find/search'
  import { t } from '../lib/i18n.svelte'

  let { text, origin, term = $bindable(''), show }: {
    text: string
    origin: number
    term?: string
    show: (m: Match | null) => void
  } = $props()

  const matches = $derived(findAll(text, term))

  let current = $state(-1)
  let active: Match | null = $state(null)
  let input: HTMLInputElement | null = null

  function run(from: number) {
    current = firstAtOrAfter(matches, from)
    active = matches[current] ?? null
    show(active)
  }

  onMount(() => {
    input?.focus()
    input?.select()
    if (term !== '') run(origin)
  })

  function onInput() {
    run(active?.start ?? origin)
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault()
      if (e.shiftKey) prev()
      else next()
    }
  }

  function next() {
    if (matches.length === 0) return
    if (current === -1) current = 0
    else current = (current + 1) % matches.length
    active = matches[current]
    show(active)
  }

  function prev() {
    if (matches.length === 0) return
    if (current === -1) current = matches.length - 1
    else current = (current - 1 + matches.length) % matches.length
    active = matches[current]
    show(active)
  }

  function focus() {
    input?.focus()
    input?.select()
  }

  export { next, prev, focus }
</script>

<aside class="find" aria-label={t('find.label')}>
  <input
    bind:this={input}
    type="text"
    spellcheck="false"
    bind:value={term}
    oninput={onInput}
    onkeydown={onKeydown}
    placeholder={t('find.label')}
    aria-label={t('find.label')}
  />
  <span class="count" aria-live="polite">
    {term === '' ? '' : matches.length === 0 ? t('find.none') : t('find.count', { current: current + 1, total: matches.length })}
  </span>
</aside>

<style>
  .find {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px 6px 6px;
    border-radius: 6px;
    color-scheme: light dark;
    background: Canvas;
    color: CanvasText;
    border: 1px solid color-mix(in srgb, CanvasText 18%, transparent);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.15);
    font: 14px 'Segoe UI Variable Text', 'Segoe UI', sans-serif;
    user-select: none;
  }

  .find input {
    min-width: 0;
    width: 220px;
    padding: 4px 8px;
    border: 1px solid color-mix(in srgb, CanvasText 18%, transparent);
    border-bottom-color: color-mix(in srgb, CanvasText 45%, transparent);
    background: Field;
    color: FieldText;
    font: inherit;
    border-radius: 4px;
  }

  .find input:focus-visible {
    outline: none;
    border-bottom: 2px solid AccentColor;
  }

  .count {
    white-space: nowrap;
    opacity: 0.7;
    font-size: 12px;
  }
</style>
