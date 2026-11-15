<script lang="ts">
  import type { Prefs } from '../lib/ipc/types/Prefs'
  import { setPrefs, resetPrefs, listFonts } from '../lib/ipc/commands'
  import { DARK, LIGHT } from '../lib/theme'
  import { LANGUAGES, t } from '../lib/i18n.svelte'

  type Numeric = 'font_size' | 'column_width' | 'relative_inset' | 'vertical_inset' | 'line_height'
  type Text = 'font_family' | 'background' | 'foreground' | 'language'

  let { prefs, onclose }: { prefs: Prefs; onclose: () => void } = $props()
  let form = $state<HTMLFormElement>()
  let installed = $state<string[]>([])
  listFonts().then(
    (all) => (installed = all),
    (err) => console.error(err),
  )

  // Literata va embebida y encabeza la lista; la actual siempre figura aunque no esté instalada.
  const fonts = $derived(
    [...new Set(['Literata', prefs.font_family, ...installed])].sort((a, b) =>
      a === 'Literata' ? -1 : b === 'Literata' ? 1 : a.toLowerCase().localeCompare(b.toLowerCase()),
    ),
  )

  $effect(() => {
    form?.querySelector<HTMLElement>('input, select')?.focus()
  })

  // Repinta con lo que devuelve Rust (el recorte manda aunque el valor no cambie).
  function paint(p: Prefs) {
    for (const key of Object.keys(p) as (keyof Prefs)[]) {
      const el = form?.elements.namedItem(key) as HTMLInputElement | HTMLSelectElement | null
      if (el) el.value = String(p[key])
    }
  }

  async function apply(request: Promise<Prefs>) {
    try {
      paint(await request)
    } catch (err) {
      console.error(err)
      paint(prefs)
    }
  }

  function numeric(key: Numeric, e: Event) {
    const value = (e.currentTarget as HTMLInputElement).valueAsNumber
    if (Number.isNaN(value)) return paint(prefs)
    void apply(setPrefs({ ...prefs, [key]: value }))
  }

  function text(key: Text, e: Event) {
    const value = (e.currentTarget as HTMLInputElement | HTMLSelectElement).value.trim()
    if (value === '') return paint(prefs)
    void apply(setPrefs({ ...prefs, [key]: value }))
  }

  function theme(colors: typeof LIGHT) {
    void apply(setPrefs({ ...prefs, ...colors }))
  }
</script>

<aside class="panel" aria-label={t('prefs.title')}>
  <header>
    <h1>{t('prefs.title')}</h1>
    <button type="button" class="cerrar" aria-label={t('prefs.close')} title={t('prefs.closeHint')} onclick={onclose}>✕</button>
  </header>

  <form bind:this={form} onsubmit={(e) => e.preventDefault()}>
    <section>
      <h2>{t('prefs.text')}</h2>
      <label for="fuente">{t('prefs.font')}</label>
      <select id="fuente" name="font_family" value={prefs.font_family} onchange={(e) => text('font_family', e)}>
        {#each fonts as font (font)}
          <option value={font}>{font}</option>
        {/each}
      </select>

      <label for="tamano">{t('prefs.size')}</label>
      <input id="tamano" name="font_size" type="number" min="8" max="48" step="1" value={prefs.font_size} onchange={(e) => numeric('font_size', e)} />

      <label for="interlineado">{t('prefs.lineHeight')}</label>
      <input id="interlineado" name="line_height" type="number" min="1" max="2" step="0.05" value={prefs.line_height} onchange={(e) => numeric('line_height', e)} />
    </section>

    <section>
      <h2>{t('prefs.columns')}</h2>
      <label for="ancho">{t('prefs.width')}</label>
      <input id="ancho" name="column_width" type="number" min="200" max="800" step="10" value={prefs.column_width} onchange={(e) => numeric('column_width', e)} />

      <label for="separacion">{t('prefs.gap')}</label>
      <input id="separacion" name="relative_inset" type="number" min="0" max="20" step="0.5" value={prefs.relative_inset} onchange={(e) => numeric('relative_inset', e)} />

      <label for="margen">{t('prefs.margin')}</label>
      <input id="margen" name="vertical_inset" type="number" min="0" max="80" step="1" value={prefs.vertical_inset} onchange={(e) => numeric('vertical_inset', e)} />
    </section>

    <section>
      <h2>{t('prefs.colors')}</h2>
      <label for="fondo">{t('prefs.background')}</label>
      <input id="fondo" name="background" type="color" value={prefs.background} onchange={(e) => text('background', e)} />

      <label for="texto">{t('prefs.foreground')}</label>
      <input id="texto" name="foreground" type="color" value={prefs.foreground} onchange={(e) => text('foreground', e)} />

      <span>{t('prefs.theme')}</span>
      <div class="temas">
        <button type="button" onclick={() => theme(LIGHT)} title={t('prefs.themeHint')}>{t('prefs.light')}</button>
        <button type="button" onclick={() => theme(DARK)} title={t('prefs.themeHint')}>{t('prefs.dark')}</button>
      </div>
    </section>

    <section>
      <h2><label for="idioma">{t('prefs.language')}</label></h2>
      <select id="idioma" name="language" value={prefs.language} onchange={(e) => text('language', e)}>
        <option value="auto">{t('prefs.auto')}</option>
        {#each LANGUAGES as [code, name] (code)}
          <option value={code} lang={code}>{name}</option>
        {/each}
      </select>
    </section>

    <footer>
      <button type="button" onclick={() => apply(resetPrefs())}>{t('prefs.reset')}</button>
    </footer>
  </form>
</aside>

<style>
  .panel {
    --line: color-mix(in srgb, CanvasText 18%, transparent);
    position: absolute;
    inset: 0 0 0 auto;
    width: 320px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    color-scheme: light dark;
    background: Canvas;
    color: CanvasText;
    border-left: 1px solid var(--line);
    box-shadow: -8px 0 24px rgb(0 0 0 / 0.18);
    font: 14px 'Segoe UI Variable Text', 'Segoe UI', sans-serif;
    user-select: none;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 4px 20px;
  }

  h1 {
    margin: 0;
    font: 600 20px 'Segoe UI Variable Display', 'Segoe UI', sans-serif;
  }

  form {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 12px 20px 20px;
  }

  section {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 128px;
    gap: 8px 12px;
    align-items: center;
  }

  h2 {
    grid-column: 1 / -1;
    margin: 0 0 2px;
    font-size: 12px;
    font-weight: 600;
    opacity: 0.7;
  }

  input,
  select,
  button {
    box-sizing: border-box;
    height: 32px;
    min-width: 0;
    font: inherit;
    border: 1px solid var(--line);
    border-radius: 4px;
  }

  input[type='number'] {
    width: 100%;
    padding: 0 8px;
    background: Field;
    color: FieldText;
    border-bottom-color: color-mix(in srgb, CanvasText 45%, transparent);
  }

  select {
    grid-column: 1 / -1;
    width: 100%;
    padding: 0 8px;
    background: Field;
    color: FieldText;
  }

  input:focus-visible,
  select:focus-visible {
    outline: none;
    border-bottom: 2px solid AccentColor;
  }

  input[type='color'] {
    width: 56px;
    padding: 3px;
    background: Field;
  }

  button {
    padding: 0 12px;
    background: ButtonFace;
    color: ButtonText;
  }

  button:hover {
    background: color-mix(in srgb, ButtonFace 90%, CanvasText);
  }

  button:focus-visible {
    outline: 2px solid AccentColor;
    outline-offset: 1px;
  }

  .cerrar {
    width: 32px;
    padding: 0;
    border-color: transparent;
    background: transparent;
  }

  .temas {
    display: flex;
    gap: 8px;
  }

  .temas button {
    flex: 1;
    padding: 0;
  }

  footer {
    margin-top: auto;
    display: flex;
    justify-content: flex-end;
  }
</style>
