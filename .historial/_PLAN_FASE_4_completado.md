# Plan de Implementación: Fase 4 — Búsqueda incremental y resaltado

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> B1–B8 las fija el orquestador por ser las de menor código y las más cercanas a Tofu; el titular puede reabrirlas antes de implementar.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| B1 | **Todo en el frontend.** Sin comandos, eventos ni cambios en Rust. El texto aplanado sale del `Doc` que ya tiene la ventana. | §10 ya lo sitúa en `lib/find/*`; el índice de carácter (D3/D9) es el mismo que usa el DOM. | todos |
| B2 | **Coincidencia literal, sin distinguir mayúsculas, sí acentos.** `RegExp(escapado, 'giu')` sobre el texto aplanado; los índices son UTF-16 del texto original (D9). Sin opciones en la UI. | Como la búsqueda por defecto de AppKit. La bandera `i` no cambia la longitud del texto (`toLowerCase` sí, p. ej. `İ`), así que no hace falta tabla de índices. Plegar acentos exigiría normalizar y mapear: sin consumidor que lo pida. | P2 |
| B3 | **Las coincidencias nunca cruzan bloques.** El término sale de un `<input type="text">`, que no admite `\n`, y el separador de bloque es `\n`. | Permite construir el `Range` dentro de un único `<p>`. | P2, P3 |
| B4 | **Sólo se resalta la coincidencia actual**, con un único `Highlight` registrado como `find` en `CSS.highlights` (`background: Highlight; color: HighlightText`). La barra muestra «n de m» o «Sin resultados». | Tofu resalta sólo la selección. Resaltar todas obliga a crear un `Range` por coincidencia (cientos de miles con una letra). El recuento sale gratis de la lista de coincidencias. | P3, P4 |
| B5 | **Origen y orden.** Al abrir la barra, `origen = anchor()` (primer carácter visible). Al buscar (cambio de término o apertura con término) se elige la primera coincidencia con `start ≥ desde`, donde `desde` = inicio del resultado resaltado **antes** de la pulsación (si lo había), si no `origen`. F3/Intro → siguiente en la lista; Mayús+F3/Mayús+Intro → anterior; ambas dan la vuelta. | Comportamiento de búsqueda incremental de navegador/AppKit: afinar el término no salta. | P2, P4 |
| B6 | **«Centrar» en columnas.** Si la columna de la coincidencia ya está entre las visibles, no se desplaza; si no, se va a `col − floor((visibles − 1) / 2)` (con el tope de `goTo`). El desplazamiento actualiza el ancla y la posición guardada por la vía normal (`scrollend` → `sync` → `onanchor`): se guarda la vista del resultado, no el resultado. | Equivale a `centerSelectionInVisibleArea` sin saltar en cada tecla cuando el resultado ya se ve. | P3 |
| B7 | **Teclas.** Ctrl+F abre la barra (cierra Preferencias si está abierta) o, si ya está abierta, enfoca y selecciona su texto. F3 / Mayús+F3: con la barra abierta, siguiente / anterior; cerrada, la abren como Ctrl+F. Intro / Mayús+Intro en el campo = F3 / Mayús+F3. Esc cierra Preferencias si está abierta; si no, la barra. Cerrar borra el resaltado y no mueve el documento. El término se conserva mientras viva la ventana. Sin documento, ninguna de estas teclas hace nada. | §7.1. Una sola regla para F3 cerrada evita un modo «buscar sin barra»; cerrar Preferencias evita que el panel tape la barra. | P2, P4 |
| B8 | **La barra es un `<aside>`** en `FindBar.svelte`, superpuesta arriba a la derecha con colores del sistema (como Preferencias). La guarda de `Reader` (`closest('aside')`) la cubre; sólo se amplía para dejar pasar F3/Mayús+F3. Al llegar un documento nuevo la barra se cierra (en `pull()`, donde se asigna `doc`). | Reutiliza la guarda y el estilo existentes; cerrar al cambiar de documento evita rangos sobre nodos desmontados. | P4 |

## 2. Objetivo

Criterio de hecho de §13 (fase 4): **encontrar, saltar y centrar un resultado.** Se construye: búsqueda incremental en cada tecla, siguiente/anterior con vuelta, resaltado con la CSS Custom Highlight API y desplazamiento por columnas al resultado.

## 3. Resumen del análisis

- **Stack:** Svelte 5.57, TS 6.0, Vitest 5.1 (proyectos `unit` y `layout` en el motor de WebView2), Tauri 2. Sin dependencias nuevas.
- **Estado heredado (`02338c9`):**
  - `Columns.svelte` pinta un `<p data-char-start>` por bloque con un nodo de texto por run; expone `go`, `goTo`, `first`, `last`, `visibleColumns`, `anchor`.
  - `columnScroll.ts` guarda la columna objetivo `k`, mide el `pitch` y, con `sync` en `scrollend`, recalcula el ancla y llama a `onAnchor` sólo si el scroll no es de restauración. `goTo` usa `'smooth'` salvo `prefers-reduced-motion`.
  - `anchor.ts`: `columnOfCharIndex` hace la búsqueda binaria en `starts`; `textBefore` y `charRect` (privados) recorren los nodos de texto de un `<p>`.
  - `textIndex.ts` tiene `blockStarts` y `flatLength`; no hay texto aplanado.
  - `keyboard.ts::actionFor` corta en `if (shiftKey) return null` antes de las teclas de navegación (F3 con Mayús debe ir antes).
  - `Reader.svelte`: `type ColumnsApi` sin `anchor` ni `show`; `doc` se asigna en `pull()`; Esc sólo cierra Preferencias; la guarda `inPanel && (action === 'paste' || !e.ctrlKey)` ignora en el panel todo lo que no lleva Ctrl. Preferencias ocupa los 320 px de la derecha a toda altura.
  - `src/lib/find/` y `FindBar.svelte` no existen.
- **Cobertura:** `keyboard.test.ts` (tabla de `actionFor`), `textIndex.test.ts` (paridad con Rust sobre `unicode.doc.json`, `starts = [0, 8, 11, 20, 21, 25]`), `tests/layout/anchor.svelte.test.ts` (helpers `setup` con `$state`, `settle`, `scrollend`, `charLeft`; `longDoc` en `defaults.ts`).
- **Riesgos:**
  1. WebView2 trae su propia búsqueda (Ctrl+F / F3). Se espera que `preventDefault()` en `keydown` la suprima (la evidencia de Ctrl+O es débil). Se comprueba en P5; si no, `create_reader` desactiva los atajos del navegador (`AreBrowserAcceleratorKeysEnabled(false)` vía `with_webview`), único cambio en Rust permitido y sólo en ese caso.
  2. Con `bind:value`, `term` y la lista de coincidencias ya son los nuevos cuando corre `oninput`: el inicio del resultado anterior debe guardarse aparte (B5, P4).
  3. Teclear durante un `scrollTo` suave: `k` ya vale el destino, así que la comprobación «ya visible» de B6 usa el destino y no hay saltos dobles.
  4. Rendimiento: `matchAll` sobre ~1 M de caracteres por tecla es de milisegundos; la medición real es de la fase 8.

## 4. Restricciones innegociables

- Nunca `pnpm build`, `pnpm tauri build` ni instaladores. `pnpm tauri dev` sí. Puertas desde PowerShell.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D3/D9 (índices UTF-16 sobre el texto aplanado), D11 (el frontend no muestra errores), §6 (el DOM no añade texto: la barra vive fuera de `.columns`).
- Sin dependencias nuevas. Sin cambios en Rust salvo el respaldo del riesgo 1.

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos (`Reader.svelte`, `Columns.svelte`, `Preferences.svelte`, `columnScroll.ts`, `anchor.ts`, `textIndex.ts`, `keyboard.ts`, sus tests, `defaults.ts`, `doc_windows.rs::create_reader`)
- [x] Evidencia de Tofu (§9.5 de `rea-tofu-evidence.md`): búsqueda incremental, `centerSelectionInVisibleArea`; el pasteboard de búsqueda no se replica (§10)
- [x] Fase 3 confirmada por el titular (`02338c9`)
- [x] Cobertura actual: Rust 77, unit 63, layout 17
- [x] `CSS.highlights`, `Highlight`/`HighlightText` y los escapes con `u` comprobados en EdgeCore 157 (auditoría)

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- `git status` limpio salvo este plan; `pnpm verify` en verde. Punto de vuelta atrás: `02338c9`.

**Gate:** `pnpm verify` = 0 sobre `02338c9`.

### Paso 2 — Búsqueda pura y teclas
**Archivos:** `src/lib/layout/textIndex.ts`, `src/lib/layout/textIndex.test.ts`, `src/lib/find/search.ts` (nuevo), `src/lib/find/search.test.ts` (nuevo), `src/lib/nav/keyboard.ts`, `src/lib/nav/keyboard.test.ts`
- `textIndex.ts`: `flatText(doc)` = runs de cada bloque concatenados, bloques unidos con `\n`; `blockAt(starts, i)` = la búsqueda binaria que hoy vive en `columnOfCharIndex` (P3 la reutiliza); `starts` vacío → `-1`.
- `search.ts`: `type Match = { start: number; end: number }`; `findAll(text, term): Match[]` (B2; término vacío → `[]`; escapa exactamente `^$\.*+?()[]{}|`: añadir `-` o `/` lanza `SyntaxError` con `u`); `firstAtOrAfter(matches, from): number` → índice de la primera con `start ≥ from`, `0` si ninguna (vuelta) y `-1` si la lista está vacía. Siguiente/anterior son aritmética de índice en P4.
- `keyboard.ts`: acciones `find` (Ctrl+F, sólo `f` minúscula), `findNext` (F3), `findPrev` (Mayús+F3); F3 se resuelve **antes** del corte por Mayús. Con Ctrl/Alt/Meta, F3 → nada.
- Tests: `flatText(unicode).length === flatLength(unicode)` (= 25); `blockAt` en `0, 7, 8, 11, 20, 21, 24` sobre los `starts` de `unicode` y `blockAt([], 0) = -1`; `findAll` sin distinguir mayúsculas, con acentos distintos que no casan, con `.`/`(`/`\` literales, con emoji (índices UTF-16) y `findAll('ab\ncd', 'bc') = []`; `firstAtOrAfter` exacto, intermedio, con vuelta y con lista vacía; filas nuevas de `actionFor` (`Ctrl+F` → `find`, `Ctrl+Mayús+F`, `Ctrl+F3`, `f` → nada; `F3` → `findNext`, `Mayús+F3` → `findPrev`).

**Gate:** `pnpm check` y `pnpm test` verdes.

### Paso 3 — Resaltado y salto a columna
**Archivos:** `src/lib/find/highlight.ts` (nuevo), `src/lib/layout/anchor.ts`, `src/lib/nav/columnScroll.ts`, `src/components/Columns.svelte`, `tests/layout/anchor.svelte.test.ts`
- `anchor.ts`: `columnOfCharIndex` usa `blockAt`; se exporta `nodeAt(p, offset, end = false): { node: Text; offset: number } | null`, que recorre los nodos de texto del `<p>` (para el inicio, el nodo con `offset < len`; con `end`, el nodo con `offset ≤ len`, de modo que un fin en el límite de un run o del bloque queda en ese nodo con su longitud). `charRect` lo usa.
- `highlight.ts`: `rangeOf(columns, starts, m): Range | null` (bloque con `blockAt`, extremos con `nodeAt`; cualquier extremo irresoluble → `null`) y `showMatch(range: Range | null)`: `CSS.highlights.set('find', new Highlight(range))` o `delete('find')`.
- `columnScroll.ts`: `reveal(i, behavior?)` según B6 (sin `pitch`, nada).
- `Columns.svelte`: `export function show(m: Match | null, behavior?: ScrollBehavior)` → `showMatch(rangeOf(...))` y, si hay `m`, `reveal(m.start, behavior)`; la limpieza de `onMount` llama a `showMatch(null)`. Estilo `:global(::highlight(find)) { background-color: Highlight; color: HighlightText; }`.
- Tests de layout: nuevo `describe('búsqueda (fase 4)')` en `anchor.svelte.test.ts` (reutiliza `setup`/`scrollend`/`charLeft` sin moverlos); los saltos con `'instant'`. `rangeOf(...).toString()` igual al texto de la coincidencia, también en un bloque de dos runs con la coincidencia partida entre ambos y con una que acaba al final del bloque; `show` de una coincidencia lejana → su carácter queda dentro del viewport, en la columna `floor((visibles − 1) / 2)` contando desde la izquierda; `show` de una ya visible no cambia `scrollLeft`; `CSS.highlights.get('find').size === 1`, `show(null)` lo borra y desmontar también.

**Gate:** `pnpm verify` verde.

### Paso 4 — Barra de búsqueda
**Archivos:** `src/components/FindBar.svelte` (nuevo), `src/components/Reader.svelte`
- `FindBar.svelte` (`<aside aria-label="Buscar">`, B8): props `text`, `origin`, `term` (`$bindable`), `show(m | null)`. `matches = $derived(findAll(text, term))`; estado propio (`$state`) `current` (índice) y `active: Match | null`, escritos sólo por `run(from)`: `current = firstAtOrAfter(matches, from)`, `active = matches[current] ?? null`, `show(active)`.
  - `oninput` → `run(active?.start ?? origin)` (B5); al montar: enfocar y seleccionar el `<input>` (como Preferencias) y, si `term !== ''`, `run(origin)`.
  - Exporta `next()` / `prev()` (`current ± 1` con vuelta sobre `matches.length`, actualiza `active`, `show`) y `focus()` (enfoca y selecciona).
  - En el campo: Intro / Mayús+Intro → `next` / `prev`, salvo `e.isComposing`. `spellcheck="false"`.
  - Muestra «n de m» / «Sin resultados» (vacío si no hay término), con `aria-live="polite"`.
- `Reader.svelte`:
  - `ColumnsApi` gana `anchor(): number` y `show(m: Match | null): void`; `FindBarApi = { next(): void; prev(): void; focus(): void }`.
  - Estado `showFind`, `findTerm`, `findOrigin`; `text = $derived(doc ? flatText(doc) : '')`.
  - Acciones (todas con `if (!doc) return`): `find` → si está abierta, `focus()`; si no, `showPrefs = false`, `findOrigin = columns?.anchor() ?? 0`, `showFind = true`. `findNext` / `findPrev` → `next` / `prev` si está abierta; si no, como `find`.
  - Esc: Preferencias primero, después la barra. Cerrar → `showFind = false` y `columns?.show(null)`.
  - Guarda: `if (inPanel && (action === 'paste' || !(e.ctrlKey || action === 'findNext' || action === 'findPrev'))) return`.
  - `pull()`: al asignar un `doc` nuevo, `showFind = false` (B8; `findTerm` se conserva).

**Gate:** `pnpm verify` verde.

### Paso 5 — Comprobación en la app
**Archivos:** ninguno (orquestador, `pnpm tauri dev`; sin verificación visual).
- Log sin errores al abrir la barra y buscar. Riesgo 1: si Ctrl+F o F3 abren la búsqueda propia de WebView2, aplicar el respaldo y repetir.
- **Titular:** con un `.txt` largo, Ctrl+F y teclear encuentra, resalta y lleva al resultado centrado; refinar el término no salta; F3 / Mayús+F3 / Intro recorren con vuelta; «Sin resultados»; Esc cierra y quita el resaltado sin mover; F3 con la barra cerrada la reabre con el término y resalta; Ctrl+F con Preferencias abierta la cierra; las flechas y Espacio escriben/mueven en el campo sin pasar columnas; en ventana vacía Ctrl+F no hace nada; con tema oscuro el resaltado se lee; reabrir el archivo vuelve a la vista del último resultado.

### Paso 6 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `REGISTRO.md`, este plan (y `DIFERIDOS.md` sólo si §9 deja de estar vacío)
- Prueba negativa (revertida): borrar la vuelta de `firstAtOrAfter` (devolver `-1` en vez de `0`) hace fallar su test unitario. `FindBar` sólo tiene la cobertura de P5.
- `ARCHITECTURE.md`: §4 (`find/search.ts`, `find/highlight.ts`, `FindBar.svelte`), §7.1 (Ctrl+F, F3, Intro, Esc según B7), §10 (B2, B4–B6), §13 (fase 4 hecha).
- `REGISTRO.md`: asiento de fase 4. Este plan → `.historial/_PLAN_FASE_4_completado.md`.
- Commit `feat: fase 4 — búsqueda incremental y resaltado`.

**Gate:** `pnpm verify` verde sobre el commit final; `git status` limpio.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5 ─→ P6
```
P3 necesita `blockAt` y `Match` (P2); P4 `findAll`/`firstAtOrAfter` (P2) y `Columns.show` (P3).

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 sobre `02338c9` (Rust 77, unit 63, layout 17) | |
| P2 — Búsqueda pura y teclas | ✅ completado | `pnpm check` 0/0; `pnpm test` = 0 (unit 82) | OpenCode `apinex/free/deepseek-v4.1-flash`. `exec` en bucle en vez de `matchAll` (target es2017); escape carácter a carácter |
| P3 — Resaltado y salto | ✅ completado | `pnpm verify` = 0 (unit 82, layout 22). Negativa: sin la comprobación «ya visible» falla su test | Revisión: corregida la condición «ya visible» de `reveal` (movía con la coincidencia en la columna 1) y su test, que usaba la columna 0; `nodeAt` duplicado en `highlight.ts` → importado de `anchor.ts` |
| P4 — Barra de búsqueda | ✅ completado | `pnpm verify` = 0 (Rust 77, unit 82, layout 22; check 0/0). Sin test propio de `FindBar` (cobertura en P5) | El agente agotó la cuota de APInex tras escribirlo; antes hizo `git stash` + `drop` (revirtió P2/P3 en archivos rastreados), tocó `ARCHITECTURE.md`/`REGISTRO.md` y movió este plan a `.historial/`. Código recuperado del commit huérfano `ce878a4`; documentos descartados; plan devuelto a la raíz. Retoques: ternarios como sentencia → `if`, relleno de la barra |
| P5 — Comprobación en la app | ✅ completado | Titular, 2026-11-12: «Funciona todo» | Ctrl+F/F3 abren la barra propia, no la de WebView2: sin respaldo en Rust. |
| P6 — Cierre y commit | ✅ completado | Negativa: `firstAtOrAfter` sin vuelta → falla «más allá de la última: vuelta al 0» | §9 vacío: sin diferidos. |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- Resaltar todas las coincidencias (B4), búsqueda sin acentos, palabra completa, expresiones regulares y reemplazar.
- Pasteboard de búsqueda del sistema y su temporizador de 1 s (§10).
- Buscar la selección (`findSelection:`): el lector no tiene selección propia.
- Recentrar el resultado tras un reflow con la barra abierta: `restore()` vuelve al ancla (primera columna visible), como cualquier otro reflow.
- Fullscreen (Esc de pantalla completa entra en la fase 5 y deberá respetar el orden de B7).

**Hallazgos aparcados**
- (vacío)

## 10. Auditoría

2026-11-12, agy y OpenCode (Big Pickle), sólo lectura y sin respaldo físico; deliberación del orquestador (Opus).

- **Aplicado:** inicio del resultado anterior guardado aparte (`active`), B5 reescrito; cierre de la barra en `pull()` en vez de un `$effect`; búsqueda y foco al montar la barra; `ColumnsApi`/`FindBarApi` y guarda sin documento; Ctrl+F cierra Preferencias; `nodeAt` exportado con regla de fin `≤`; `blockAt([])` = `-1`; `:global(::highlight(find))`; tests en `anchor.svelte.test.ts` sin mover helpers (`$state` exige `.svelte.ts`) y con `'instant'`; tests de `blockAt`/`findAll` que prueban lo que dicen; prueba negativa ejecutable; `isComposing` y `spellcheck`; «vista del último resultado»; recentrado tras reflow excluido; `DIFERIDOS.md` sólo si hace falta.
- **Simplificado:** `pick` con tres modos → `firstAtOrAfter` + aritmética de índice; fuera Ctrl+Mayús+F.
- **Rechazado:** mover los helpers de layout a un módulo compartido (un solo archivo de tests basta).
