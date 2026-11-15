<script lang="ts">
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import type { Doc } from '../lib/ipc/types/Doc'
  import type { Prefs } from '../lib/ipc/types/Prefs'
  import {
    getPrefs,
    openDialog,
    openFromClipboard,
    saveCopy,
    setPosition,
    setPrefs,
    takePendingDocument,
  } from '../lib/ipc/commands'
  import { toggledTheme } from '../lib/theme'
  import { effective, setLang } from '../lib/i18n.svelte'
  import { onDocumentReady, onPrefsChanged } from '../lib/ipc/events'
  import { actionFor, nextScale } from '../lib/nav/keyboard'
  import { wheelZoom } from '../lib/nav/wheel'
  import Columns from './Columns.svelte'
  import EmptyState from './EmptyState.svelte'
  import FindBar from './FindBar.svelte'
  import Preferences from './Preferences.svelte'
  import { flatText } from '../lib/layout/textIndex'
  import type { Match } from '../lib/find/search'

  type ColumnsApi = {
    go(delta: number): void
    first(): void
    last(): void
    visibleColumns(): number
    anchor(): number
    show(m: Match | null): void
  }

  type FindBarApi = {
    next(): void
    prev(): void
    focus(): void
  }

  let doc = $state<Doc | null>(null)
  let checked = $state(false)
  // Inyectadas por Rust al crear la ventana (doc_windows::create_reader); getPrefs sólo reconcilia.
  let prefs = $state<Prefs | null>((window as { __PREFS__?: Prefs }).__PREFS__ ?? null)
  let scale = $state(1)
  let showPrefs = $state(false)
  let columns = $state<ColumnsApi | undefined>(undefined)
  let findBar = $state<FindBarApi | undefined>(undefined)
  let showFind = $state(false)
  let findTerm = $state('')
  let findOrigin = $state(0)
  let fullscreen = $state(false)
  const text = $derived(doc ? flatText(doc) : '')

  $effect(() => {
    if (prefs) setLang(effective(prefs.language))
  })

  function openFind() {
    showPrefs = false
    findOrigin = columns?.anchor() ?? 0
    showFind = true
  }

  function closeFind() {
    showFind = false
    columns?.show(null)
  }

  // El estado sólo cambia si la llamada resuelve.
  function setFullscreen(next: boolean) {
    void getCurrentWindow()
      .setFullscreen(next)
      .then(
        () => (fullscreen = next),
        (err) => console.error(err),
      )
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && showPrefs) {
      e.preventDefault()
      showPrefs = false
      return
    }
    if (e.key === 'Escape' && showFind) {
      e.preventDefault()
      closeFind()
      return
    }
    if (e.key === 'Escape' && fullscreen) {
      e.preventDefault()
      setFullscreen(false)
      return
    }
    const action = actionFor(e)
    if (!action) return
    const inPanel = e.target instanceof Element && e.target.closest('aside') !== null
    const passes = e.ctrlKey || action === 'findNext' || action === 'findPrev' || action === 'fullscreen'
    if (inPanel && (action === 'paste' || !passes)) return
    e.preventDefault()
    switch (action) {
      case 'prev':
        columns?.go(-1)
        break
      case 'next':
        columns?.go(1)
        break
      case 'pagePrev':
        columns?.go(-(columns.visibleColumns()))
        break
      case 'pageNext':
        columns?.go(columns.visibleColumns())
        break
      case 'first':
        columns?.first()
        break
      case 'last':
        columns?.last()
        break
      case 'open':
        void openDialog()
        break
      case 'save':
        void saveCopy()
        break
      case 'paste':
        void openFromClipboard()
        break
      case 'zoomIn':
      case 'zoomOut':
      case 'zoomReset':
        scale = nextScale(scale, action)
        break
      case 'prefs':
        showPrefs = !showPrefs
        break
      case 'fullscreen':
        setFullscreen(!fullscreen)
        break
      case 'theme':
        if (prefs) void setPrefs({ ...prefs, ...toggledTheme(prefs.background) })
        break
      case 'find':
        if (!doc) break
        if (showFind) findBar?.focus()
        else openFind()
        break
      case 'findNext':
        if (!doc) break
        if (showFind) findBar?.next()
        else openFind()
        break
      case 'findPrev':
        if (!doc) break
        if (showFind) findBar?.prev()
        else openFind()
        break
    }
  }

  // Listener propio: `passive: false` para poder cancelar el Ctrl+rueda.
  $effect(() => {
    const onWheel = (e: WheelEvent) => {
      const action = wheelZoom(e)
      if (!action) return
      e.preventDefault()
      scale = nextScale(scale, action)
    }
    window.addEventListener('wheel', onWheel, { passive: false })
    return () => window.removeEventListener('wheel', onWheel)
  })

  $effect(() => {
    let active = true
    let unlisten: UnlistenFn[] = []
    let changed = false
    const pull = () => {
      void takePendingDocument().then((pending) => {
        if (!active) return
        if (pending) {
          doc = pending
          showFind = false
        }
        checked = true
      })
    }

    void (async () => {
      const stops = await Promise.all([
        onDocumentReady(pull),
        onPrefsChanged((next) => {
          changed = true
          prefs = next
        }),
      ])
      if (!active) {
        stops.forEach((stop) => stop())
        return
      }
      unlisten = stops
      pull()
      void getPrefs().then((loaded) => {
        if (active && !changed) prefs = loaded
      })
    })()

    return () => {
      active = false
      unlisten.forEach((stop) => stop())
    }
  })
</script>

<svelte:window onkeydown={onKeydown} />

{#if prefs}
  <div
    class="reader"
    style:background={prefs.background}
    style:color={prefs.foreground}
    style:font-family={`"${prefs.font_family}", serif`}
    style:font-size="{prefs.font_size}px"
  >
    {#if doc}
      {#key doc}
        <Columns {doc} {prefs} {scale} {fullscreen} onanchor={(i) => void setPosition(i)} bind:this={columns} />
      {/key}
    {:else if checked}
      <EmptyState />
    {/if}
    {#if showFind && doc}
      <FindBar
        bind:this={findBar}
        {text}
        origin={findOrigin}
        bind:term={findTerm}
        show={(m) => columns?.show(m)}
      />
    {/if}
    {#if showPrefs}
      <Preferences {prefs} onclose={() => (showPrefs = false)} />
    {/if}
  </div>
{/if}

<style>
  .reader {
    position: relative;
    height: 100%;
  }
</style>
