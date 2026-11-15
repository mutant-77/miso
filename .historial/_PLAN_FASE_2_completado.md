# Plan de Implementación: Fase 2 — Ancla, viewport exacto, prefs y atajos

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> **H1–H4 cierran asuntos que eran del titular** (`_CIERRE_FASE_1.md` §2 y `DIFERIDOS.md`): son propuestas del orquestador y **requieren su visto bueno antes de P2**.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| H1 | **↑/↓ = ←/→** (una columna atrás/adelante), sin caso especial de selección. | "Media columna + ajuste al inicio más cercano" se anula a sí mismo (vuelve a la misma columna o salta a la siguiente). La columna es la unidad de navegación; el lector no tiene cursor, así que la selección no se extiende por teclado de forma nativa. | P5 |
| H2 | **UI sólo en español, sin i18n.** El texto de E/S del sistema se muestra sin el sufijo ` (os error N)`. | Un único usuario y un único `match` de textos (`user_message`); i18n sería infraestructura sin consumidor. El mensaje del sistema ya llega en el idioma de Windows. | P3 |
| H3 | **Restricción de comandos: comprobación de *label* sólo en `set_prefs` y `reset_prefs`** (`"prefs"`; si no, `Internal`). Sin `AppManifest`. | Son los únicos comandos con efecto global; el resto ya es inocuo desde cualquier ventana (D10, CSP, sin HTML ajeno). Dos líneas frente a manifiesto + permisos por comando. | P3 |
| H4 | **Rangos de prefs** (fuera de rango → recorte; cadena inválida → default del campo): fuente 8–48 px; ancho de columna 200–800 px; inset horizontal 0–20 %; inset vertical 0–80 px; interlineado 1.0–2.0; colores `#rrggbb`; familia de fuente = **un solo nombre**, 1–100 caracteres sin `" ; { } < > \` (el frontend compone `"<nombre>", serif`). | Equivalente de `ensureValidColumnWidth` para todos los campos; recortar en vez de rechazar mantiene la ventana de prefs simple. | P3 |
| H5 | **Ancla deseada** (`_desiredFirstVisibleCharacterIndex` de Tofu): se actualiza sólo en `scrollend` provocado por el usuario (teclas, rueda, barra); los desplazamientos de **restauración** no la tocan. | Si cada restauración recalculara el ancla, un arrastre de redimensionado (decenas de reflows) la haría retroceder una columna por reflow. | P4 |
| H6 | **`set_prefs` recibe `Prefs` completo** (no parcial) y devuelve el `Prefs` validado. | La ventana de prefs ya tiene el objeto entero; un tipo parcial duplicaría `Prefs` con `Option`s. | P3, P6 |
| H7 | **Prefs de fase 2:** familia y tamaño de fuente, ancho de columna, insets horizontal y vertical, interlineado, colores de fondo y texto, y partición de palabras **sólo si P2 da positivo** (provisional hasta la confirmación en la app, P7; si allí falla, se quita antes de P8). `ForcePreferredFont`, `ForceForegroundColor`, `ForceAlignment` y `ShowFullscreenMessage` entran con su consumidor (fase 5). | Cada campo entra con la fase que lo usa (como G4 de fase 1). | P2, P3 |
| H8 | **Si `hyphens: auto` no funciona en WebView2, no hay pref de partición** (ni aviso en Preferencias). | Una casilla sin efecto más un aviso que la explique es UI que no aporta nada. | P2 |
| H9 | **Escala de fuente por ventana: multiplicativa, pasos de ×1.1, rango 0.5–3, no persiste.** Afecta sólo al tamaño de fuente. | §7.1; el ancho de columna sigue siendo el de prefs (como `increaseFontSize:`). | P5 |
| H10 | **Prefs en el frontend sin módulo de estado ni `tokens.css`:** `Reader` las carga y escucha; `Columns` las recibe por *props* y las vuelca a *custom properties* en su raíz. | `state/prefs.svelte.ts` y `theme/tokens.css` no tendrían más consumidor que `Reader`. | P4 |
| H12 | **Temas Claro y Oscuro como dos botones en Preferencias** que rellenan fondo y texto de una vez (Claro = `#ffffff`/`#000000`; Oscuro = `#1e1e1e`/`#d4d4d4`). No es un modo automático: no sigue al sistema ni añade campos a `Prefs`. Decisión del titular (2026-11-12). | Leer en oscuro sin elegir dos colores a mano; sin duplicar las prefs de color. | P7 |
| H11 | **Atajos de §7.1 en fase 2: sólo Ctrl+, y Ctrl+= / Ctrl+− / Ctrl+0.** Ctrl+V (fase 3), Ctrl+F/F3 (4), F11/Esc (5) y Ctrl+S (7) llegan con su función. | Un atajo sin función detrás no se puede probar. | P5 |

## 2. Objetivo

Criterio de hecho de §13 (fase 2): **cambiar cualquier pref, la escala o el tamaño de la ventana conserva el primer carácter visible; 500 saltos de columna sin error acumulado (tests de layout).** Se construye: prefs persistentes con ventana propia, viewport de ancho múltiplo exacto, ancla con restauración tras reflow, ↑/↓, escala de fuente por ventana, Ctrl+, y comprobación de `hyphens: auto`.

## 3. Resumen del análisis

- **Estado heredado (`cceeae7`):** ventanas `reader-{n}` creadas por Rust (`doc_windows::create_reader`, 1150×640 desde el ajuste del 2026-11-12 —3 columnas por defecto—, mín. 480×360); comandos `take_pending_document` y `open_dialog`; `AppError { Read, Unsupported, Internal }` + `report()`; `Columns.svelte` con valores fijos (columna 350 + gap 31.6 = paso 381.6, padding 20/15.8, Palatino 14 px, interlineado `calc(1.1 × 1lh)` sobre `line-height: normal`: equivalencia con AppKit de ARCHITECTURE §6) y columnas **estiradas** al ancho; `columnScroll.ts` (`go`, `goTo(k, behavior?)`, `first`, `last`, `visibleColumns`, `sync` en `scrollend`, `remeasure` sin ancla); `textIndex.ts` + `data-char-start` por `<p>`; `App.svelte` monta `Reader` sin mirar el *label*; `capabilities/reader.json` con `core:event:default` + `core:window:default`.
- **Pendiente heredado:** confirmación visual del titular de la fase 1 (`_CIERRE_FASE_1.md` §1). Bloquea P2.
- **Riesgos:**
  1. Las restauraciones generan `scrollend` → sin H5 el ancla deriva.
  2. Un `ResizeObserver` que dispara varias veces por arrastre → la restauración debe ser idempotente (mismo ancla → misma columna).
  3. `21.6 px` no es representable en unidades de layout de Chromium (1/64 px): el `pitch` se sigue **midiendo**, nunca se calcula desde la pref (§6). El test de 500 saltos lo cubre.
  4. WebView2 tiene zoom propio en Ctrl+=/−/0 y Ctrl+rueda: Tauri lo desactiva por defecto (`zoom_hotkeys_enabled` = `false`); además `preventDefault` en el `keydown`. Se comprueba en P5.
  5. `hyphens: auto` necesita `lang` y diccionarios; en Windows Chromium los descarga como componente y WebView2 puede no tenerlos (§6 *verificar*).
  6. `Range` colapsado puede no dar rectángulos: el ancla se localiza con un rango de **un carácter**.
  7. `prefs-changed` se emite con `app.emit` (a todos) y cada lector escucha con `getCurrentWebviewWindow().listen`: comprobar en P7 que lo recibe.
  8. *Scroll anchoring* nativo de Chromium (`overflow-anchor: auto`) puede mover `scrollLeft` en un reflow y producir un `scrollend` que no es del usuario → `overflow-anchor: none` en `.viewport`.
- **Auditoría (2026-11-11, agy + OpenCode Big Pickle, sólo lectura, sin respaldo físico):** aplicados `--vp-w` derivado de prefs (no sólo del *resize*), `remeasure` sin `sync`, `restoring` por comparación antes/después, `overflow-anchor: none`, separador de bloque en `columnOfCharIndex`, `fitViewport` sin `inset` (`gap = 2 × inset` lo anula), `set_prefs` sin `Err` por persistencia, el formulario nunca envía `NaN`, `{#key doc}` en lugar de reiniciar el ancla, carrera `getPrefs`/`prefs-changed`, `open_prefs` con `unminimize` y doble pulsación, sonda de `hyphens` siempre temporal, sin `capabilities/prefs.json`, `nextScale` puro con test, erratas de pasos y archivos omitidos. Auditoría de interfaz (Opus, Claude Code): aplicados color de fondo nativo de la ventana desde prefs, fuente heredada en el estado vacío, familia como nombre único con respaldo `serif`, rótulos y pasos de la ventana de prefs, barra de desplazamiento con el color del texto, formulario sin pintar hasta tener prefs, Esc cierra Preferencias y comprobaciones de teclado español, modo oscuro y partición de un texto en inglés. Rechazado: colocar Preferencias junto al lector de origen (cálculo de geometría por una molestia que el usuario resuelve moviendo la ventana). Rechazados (agy/OpenCode): "`.svelte.test.ts` no compila runas" (el plugin compila todo módulo con infijo `.svelte.`, `constants.js:56`); "`devicePixelRatio` no cambia con el zoom" (en Chromium sí; se mantiene junto a `innerWidth`).

## 4. Restricciones innegociables

- Nunca `pnpm build`, `pnpm tauri build` ni instaladores. `pnpm tauri dev` sí. Compilación y puertas **desde PowerShell**.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D8 (px), D9 (UTF-16, el DOM no añade texto), D10 (ningún comando recibe rutas), D11 (el frontend no muestra errores).
- Sin dependencias nuevas (ni Rust ni npm).
- Escuchar eventos con `getCurrentWebviewWindow().listen`, nunca con el `listen` global. Comandos que crean ventanas, `async`.

## 5. Checklist previo a los pasos

- [x] Estructura del proyecto revisada
- [x] Archivos afectados leídos (`lib.rs`, `commands.rs`, `doc_windows.rs`, `error.rs`, `model.rs`, `tauri.conf.json`, `reader.json`, `App.svelte`, `Reader.svelte`, `Columns.svelte`, `EmptyState.svelte`, `columnScroll.ts`, `keyboard.ts`, `wheel.ts`, `textIndex.ts`, `ipc/*`, `columns.test.ts`, `vitest.config.ts`, `package.json`)
- [x] Patrones: los de fase 1 (tipos `ts-rs`, `report()`, `Slots` puro y testeable)
- [x] H1–H4 y la fuente por defecto como nombre único (`Palatino Linotype` + `serif`) confirmadas por el titular (2026-11-12)
- [x] Confirmación visual de la fase 1 por el titular (2026-11-12): columnas, navegación, Ctrl+O en ventana llena y vacía, avisos `Read` y `Unsupported` modales sobre su ventana
- [x] Cobertura de pruebas mapeada (Rust 21, unit 21, layout 4)
- [x] Consumidores: `Columns` (Reader, tests de layout), `actionFor` (Reader, tests)

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- Confirmación visual del titular de la fase 1 y visto bueno a H1–H4. Si algo de fase 1 falla, se corrige antes (fuera de este plan).
- `git status` (sólo `_CIERRE_FASE_1.md` y este plan sin seguimiento) y `pnpm verify` en verde. Punto de vuelta atrás: `cceeae7`.

**Gate:** `pnpm verify` = 0 sobre `cceeae7`; H1–H4 confirmadas.

### Paso 2 — Sonda de `hyphens: auto` (decide H7/H8)
**Archivos:** `tests/layout/hyphens.test.ts` (temporal: se borra tras anotar el resultado)
- Test en el motor de WebView2 (proyecto `layout`): dos `<div lang="es" style="width: 60px; font: 14px serif">` con la misma palabra larga (`electroencefalografista`), uno con `hyphens: auto` y otro con `hyphens: manual`; hay partición si el primero tiene más líneas (`getClientRects()` de su texto) o algún fragmento termina antes del borde. Repetir con `lang="en"` y `hyphenation`.
- Resultado → tabla de seguimiento; el test se borra en ambos casos (depende de diccionarios descargables, no del código). **Positivo:** `hyphenation: bool` entra en `Prefs` (P3). **Negativo:** no hay pref (H8) y §6 de ARCHITECTURE lo anota.
- Si es positivo en Edge, se confirma en la app (P7) con un párrafo de palabras largas: lo comprueba el titular (EdgeCore ≠ runtime de WebView2 en componentes descargados). Si allí falla, se quita `hyphenation` de `Prefs`, `Columns` y `Preferences` antes de P8.

**Gate:** resultado anotado; decisión H7/H8 resuelta.

### Paso 3 — Prefs en Rust: modelo, persistencia, comandos y ventana
**Archivos:** `src-tauri/src/prefs.rs` (nuevo), `src-tauri/src/error.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/ipc/types/Prefs.ts` (generado)
- `prefs.rs`:
  - `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TS)] #[ts(export)] #[serde(default)] struct Prefs { font_family: String, font_size: f64, column_width: f64, relative_inset: f64, vertical_inset: f64, line_height: f64, background: String, foreground: String }` (+ `hyphenation: bool` si P2 fue positivo). `Default` = §8: `Palatino Linotype`, 14, 360, 3, 20, 1.1, `#ffffff`, `#000000` (`hyphenation: true`).
  - `fn validated(self) -> Self`: recorte por H4; cadena inválida → default del campo. (Un campo de tipo erróneo en el archivo hace fallar la lectura entera → rama de `load` de abajo.)
  - `fn load(path) -> Prefs`: no existe → default; error de lectura o de `serde_json` → default, renombrar a `prefs.json.corrupt` (sobrescribe uno anterior) y `log::warn!`. Siempre pasa por `validated`.
  - `fn save(path, &Prefs) -> io::Result<()>`: `create_dir_all` del padre, escribir `prefs.json.tmp` y `fs::rename` sobre `prefs.json` (atómico en el mismo volumen).
  - `struct PrefsState { prefs: Mutex<Prefs>, path: PathBuf, warned: AtomicBool }`; ruta `app_data_dir()/prefs.json`. Se carga en `setup` **antes** de crear `reader-0`.
  - `fn apply(app, new: Prefs) -> Prefs` (sin `Result`): valida, guarda en memoria, `save` (fallo → `Persist`: `report` sólo si `warned` pasa de `false` a `true`; siempre al log), `set_background_color` de cada ventana `reader-*` (evita el destello del fondo nativo al redimensionar), `app.emit("prefs-changed", &prefs)` y devuelve el validado.
  - `create_reader` crea la ventana con `.background_color(prefs.background)` (sin destello blanco al abrir con fondo oscuro). Un fallo de escritura **no** impide aplicar el cambio en la sesión ni hace fallar el comando.
- `error.rs`: variante `Persist { source: io::Error }` (`kind` `"persist"`, texto `No se pudieron guardar las preferencias: {mensaje}`). `fn os_message(&io::Error) -> String` que quita el sufijo ` (os error N)`; lo usan `Read` y `Persist` (H2).
- `commands.rs` (H3, H6):
  - `get_prefs() -> Prefs`.
  - `set_prefs(window, prefs: Prefs) -> Result<Prefs, AppError>` y `reset_prefs(window) -> Result<Prefs, AppError>` (= `apply(Prefs::default())`): el único `Err` es `window.label() != "prefs"` → `Internal` (sin diálogo: no es un error del usuario; sólo log).
  - `async open_prefs(app)`: si existe la ventana `prefs`, `unminimize` + `set_focus`; si no, `WebviewWindowBuilder` con label `prefs`, título `Preferencias`, 420×520, no redimensionable. Si `build` falla pero la ventana ya existe (doble Ctrl+, simultáneo), se enfoca; si no, `report` con `Internal`.
- Sin `capabilities/prefs.json`: la ventana sólo invoca comandos propios, que no pasan por la ACL (se comprueba en el gate de P7).
- `error.rs`: `Persist` entra en `variants()` y en `serializa_a_kind_y_message` (`"persist"`).
- Tests: defaults = §8; `validated` recorta cada campo en ambos extremos y repara colores y familias inválidas; un JSON parcial (`{"font_size": 18}`) carga con el resto por defecto; `load` de un JSON roto → default y existe `prefs.json.corrupt`; `save` + `load` = ida y vuelta; `os_message` sin sufijo. Archivos en un subdirectorio único de `std::env::temp_dir()`.

**Gate:** `pnpm lint:rust` y `pnpm test:rust` en verde; existe `src/lib/ipc/types/Prefs.ts` y una segunda ejecución no lo cambia.

### Paso 4 — Frontend: prefs aplicadas y viewport exacto
**Archivos:** `src/lib/ipc/commands.ts`, `src/lib/ipc/events.ts`, `src/lib/layout/columnWidth.ts` (nuevo), `src/lib/layout/columnWidth.test.ts` (nuevo), `src/components/Reader.svelte`, `src/components/Columns.svelte`, `src/components/EmptyState.svelte`, `src/App.svelte`, `tests/layout/columns.test.ts`
- `ipc`: `getPrefs`, `setPrefs`, `resetPrefs`, `openPrefs`; `onPrefsChanged(cb)` con `getCurrentWebviewWindow().listen`.
- `columnWidth.ts`: `fitViewport(available, pitch): number` = `min(available, max(1, floor(available / pitch)) × pitch)` con `pitch = column_width + 2 × inset`. Como `gap = 2 × padding-inline`, el padding de los extremos completa el último paso: el viewport es exactamente N pasos. La regla √2 es fase 5.
- `columnWidth.test.ts`: tabla (1, 2 y N columnas; `available` menor que un paso → `available`; límite exacto entre N y N+1).
- `Reader.svelte`: no pinta nada hasta tener prefs. Al montar: `onDocumentOpened`, `onPrefsChanged`, después `getPrefs` y `takePendingDocument`; si llega un `prefs-changed` antes de que resuelva `getPrefs`, se descarta la respuesta de `getPrefs`. Pasa `prefs` y `scale = 1` (P6 lo hace `$state`) a `Columns`, envuelto en `{#key doc}` (documento nuevo = scroller y ancla nuevos). Fondo, color de texto, familia y tamaño de `.reader` desde prefs.
- `EmptyState.svelte`: sin `color` ni fuente fijos (hereda de `.reader`); tono secundario con `opacity: 0.6`.
- `Columns.svelte`: props `doc`, `prefs`, `scale`. `inset = relative_inset × 0.01 × column_width`; equivalencia con AppKit (ARCHITECTURE §6): `--col-w = column_width − 10`, `--pad = inset + 5`, `--gap = 2 × inset + 10`. Custom properties en `.frame` (`--col-w`, `--gap`, `--pad`, `--v-inset`, `--font = "<font_family>", serif`, `--size = font_size × scale`, `--lh` → `line-height: calc(var(--lh) × 1lh)` sobre `line-height: normal` en `.viewport`, `--fg`). `frameWidth` en `$state` desde el `ResizeObserver` de `.frame`; `vpWidth = $derived(fitViewport(frameWidth, column_width + 2 × inset))` → `--vp-w` (así un cambio de pref sin redimensionar también lo recalcula). Estructura `.frame` (100 %) > `.viewport` (`width: var(--vp-w); margin-inline: auto; overflow-anchor: none; scrollbar-width: thin; scrollbar-color: color-mix(in srgb, var(--fg) 35%, transparent) transparent`) > `.columns`. `lang` de `.columns` = `navigator.language` (y `hyphens: auto` si existe la pref y está activa).
- `App.svelte`: el fondo global sale de `.reader`; `html/body` sin color fijo.
- `columns.test.ts`: los montajes existentes pasan prefs por defecto y `scale: 1`.

**Gate:** `pnpm verify` en verde. Layout (en `columns.test.ts`), en un host de 1000 px: con prefs por defecto y tras cambiar `column_width` a 300, `measurePitch` = `column_width + 2 × inset` (±0.5 px) y el ancho de `.viewport` = `fitViewport(...)`.

### Paso 5 — Ancla y restauración tras reflow
**Archivos:** `src/lib/layout/anchor.ts` (nuevo), `src/lib/nav/columnScroll.ts`, `src/components/Columns.svelte`, `tests/layout/anchor.svelte.test.ts` (nuevo)
- `anchor.ts`:
  - `firstVisibleCharIndex(viewport, columns): number`: `caretPositionFromPoint` en (borde izquierdo del viewport + padding izquierdo + 1, borde superior + padding superior + media línea); si `offsetNode` es texto, sube al `<p data-char-start>` y suma las longitudes de los nodos de texto anteriores dentro del `<p>` más el `offset`; si no es texto (p. ej. un `<p>` vacío), usa el `data-char-start` del `<p>` más cercano (`closest`). Si no hay `<p>`, el ancla no cambia.
  - `columnOfCharIndex(i, starts, viewport, columns, pitch): number`: `<p>` con el mayor `data-char-start` ≤ `i` (búsqueda binaria sobre `starts`), rango de **un carácter** en ese desplazamiento (riesgo 6). Si el desplazamiento es ≥ longitud del `<p>` (separador de bloque o `<p>` vacío), se usa el **último** rectángulo del `<p>`. `k = floor((rect.left − viewport.left + scrollLeft − padding izquierdo + 1) / pitch)`.
- `columnScroll.ts`: estado `anchor` (inicial 0) y `restoring: boolean`.
  - `remeasure()` sólo mide `pitch`: no llama a `sync` ni toca `scrollLeft` (si no, recalcularía el ancla tras el reflow y antes de restaurar).
  - `restore()`: `remeasure()`; `const before = viewport.scrollLeft`; `goTo(columnOfCharIndex(anchor, …), 'instant')`; `restoring = viewport.scrollLeft !== before` (sólo hay `scrollend` si se movió).
  - `sync` (en `scrollend`): recalcula `k`; si `restoring`, lo apaga; si no, `anchor = firstVisibleCharIndex()` (H5).
- `Columns.svelte`: `restore()` en el `ResizeObserver` del viewport y en un `$effect` sobre `prefs`, `scale` y `vpWidth` (corre tras actualizar el DOM). Expone `anchor()` para los tests.
- `anchor.svelte.test.ts` (props con `$state` para poder cambiarlas tras `mount`; texto largo, host 1000×700): tras `goTo(37, 'instant')` y esperar `scrollend`, para cada cambio — `font_size`, `column_width`, `line_height`, `relative_inset`, `vertical_inset`, `scale` y ancho del host — el carácter del ancla queda en la **primera columna visible** (su rectángulo entre el borde izquierdo del viewport y `+ pitch`). Sin deriva: 20 cambios alternos de `font_size` (14 ↔ 18) dejan `anchor()` igual al valor inicial. **500 saltos:** con un documento de ≥ 501 columnas (se comprueba antes de saltar), `goTo(k, 'instant')` para k = 1..500 deja el primer rectángulo visible a < 1 px del borde izquierdo + padding.

**Gate:** `pnpm verify` en verde, con los tests anteriores. Prueba negativa (revertida): quitar la guarda `restoring` hace fallar el test de deriva.

### Paso 6 — Teclado: ↑/↓, escala y Ctrl+,
**Archivos:** `src/lib/nav/keyboard.ts`, `src/lib/nav/keyboard.test.ts`, `src/components/Reader.svelte`
- `keyboard.ts`: `ArrowUp` → `prev`, `ArrowDown` → `next` (H1). Con Ctrl (sin Alt ni Meta, Mayús permitida): `=`/`+` → `zoomIn`, `-` → `zoomOut`, `0` → `zoomReset`, `,` → `prefs`.
- `keyboard.ts`: `nextScale(scale, action): number` pura (H9: ×1.1 o ÷1.1 con recorte a 0.5–3; `zoomReset` → 1).
- `Reader.svelte`: `scale` en `$state` actualizado con `nextScale`; `prefs` → `openPrefs()`.
- `keyboard.test.ts`: filas nuevas, incluido que `Alt+ArrowUp` no hace nada y que `Ctrl+Shift+=` sí es `zoomIn`; `nextScale` en ambos topes y el reset.

**Gate:** `pnpm verify` en verde. En `pnpm tauri dev` (orquestador, sin verificación visual): Ctrl+= y Ctrl+rueda no activan el zoom del WebView (`devicePixelRatio` y `window.innerWidth` sin cambios, leídos por consola/log).

### Paso 7 — Ventana de preferencias
**Archivos:** `src/App.svelte`, `src/components/Preferences.svelte` (nuevo)
- `App.svelte`: `getCurrentWebviewWindow().label === 'prefs'` → `Preferences`; si no, `Reader`.
- `Preferences.svelte`: no pinta el formulario hasta tener `getPrefs`. Formulario nativo con `<label for>` por control: **Fuente** (`<input list>` + `datalist` con Palatino Linotype, Georgia, Cambria, Constantia, Segoe UI, Calibri), **Tamaño (px)** (paso 1), **Ancho de columna (px)** (paso 10), **Separación entre columnas (% del ancho de columna)** (`relative_inset`, paso 0.5), **Margen superior e inferior (px)** (paso 1), **Interlineado** (paso 0.05) — `type="number"` con `min`/`max` de H4 —, **Fondo** y **Texto** (`type="color"`) con dos botones al lado, **Claro** y **Oscuro** (H12: un solo `setPrefs` con ambos colores), **Partir palabras con guion** (si existe) y botón **Restablecer** (`resetPrefs`). Esc cierra la ventana. Cada control aplica en `change` con `setPrefs` del objeto completo y repinta con el `Prefs` devuelto (el recorte de Rust manda). Un número vacío o no válido no se envía (`NaN` llegaría como `null` y serde rechazaría todo el objeto): se repinta el valor actual. `color-scheme: light dark` con colores del sistema.

**Gate:** `pnpm verify` en verde. En `pnpm tauri dev`: Ctrl+, abre `Preferencias` y carga sus valores sin capability propia (si `invoke` falla por ACL, se añade `capabilities/prefs.json` mínimo y se anota); un segundo Ctrl+, la enfoca sin abrir otra; cambiar el tamaño escribe `prefs.json` en `%APPDATA%\com.provisional.miso\` (leído en disco) y el log no tiene errores. Comprobación visual (titular): con dos lectores abiertos, cada cambio se aplica en caliente a ambos conservando el primer carácter visible; Restablecer vuelve a los valores de §8; los botones Claro y Oscuro cambian ambos colores a la vez en todos los lectores; con fondo oscuro, abrir y redimensionar un lector no muestra blanco y la barra de desplazamiento se lee; con Windows en modo oscuro, Preferencias tiene controles y barra de título oscuros; en teclado español funcionan Ctrl++ (también el del teclado numérico), Ctrl+Mayús+0, Ctrl+− y Ctrl+0; si hay partición, un párrafo en inglés con el sistema en español no se parte mal (si se parte mal: la pref entra desactivada por defecto, o se acepta y va a `DIFERIDOS.md` ligado a `Doc.lang`, fase 5).

### Paso 8 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, este plan, `_CIERRE_FASE_1.md`
- Prueba negativa (revertida): un rango de H4 roto (p. ej. `column_width` sin recorte superior) hace fallar `test:rust`.
- `ARCHITECTURE.md`: §4 (sin `state/` ni `theme/`, sin `capabilities/prefs.json` salvo que P7 lo exija); §5 (`set_prefs(Prefs)` completo, `open_prefs`, `Persist`, comprobación de *label*); §6 (capa `.frame` y `--vp-w` derivado de prefs + ancho de `.frame`, `overflow-anchor: none`, ancla deseada, `fitViewport`, resultado de `hyphens`); §7 (↑/↓ = ←/→); §8 (rangos de H4; la fuente por defecto pasa a ser el nombre `Palatino Linotype` con respaldo `serif`); §11 (H3); §13 (fase 2 hecha según lo entregado).
- `DIFERIDOS.md`: cerrar rangos de prefs, idioma de la UI y restricción de comandos; añadir lo aparcado en §9 que siga abierto, con fecha y hora.
- `REGISTRO.md`: asiento de fase 2.
- Este plan → `.historial/_PLAN_FASE_2_completado.md`. `_CIERRE_FASE_1.md` → `.historial/.informes/_CIERRE_FASE_1_cumplido.md`.
- Commit `feat: fase 2 — ancla, viewport exacto, prefs y atajos`.

**Gate:** `pnpm verify` verde sobre el commit final; `git status` limpio; ni el plan ni el cierre siguen en la raíz.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5 ─→ P6 ─→ P7 ─→ P8
```
P3 depende de P2 (¿campo `hyphenation`?); P4 de los tipos y comandos (P3); P5 del viewport y las prefs por *props* (P4); P6 de `scale` en `Columns` (P4) y de `open_prefs` (P3); P7 de los comandos (P3).

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 sobre `c3b1a7f` (Rust 21, unit 21, layout 4) | H1–H4 confirmadas en §5 |
| P2 — Sonda de `hyphens` | ✅ completado | EdgeCore 157: auto = manual = 1 línea (es, en), también tras 8 s | Negativo → H8: sin pref; sonda borrada |
| P3 — Prefs en Rust | ✅ completado | clippy y `test:rust` (33) verdes; `Prefs.ts` estable | `serde_json` a dependencias normales; revertido el re-aviso de `Persist` |
| P4 — Prefs aplicadas y viewport exacto | ✅ completado | layout: pitch y ancho de viewport para 360 y 300 px | `--vp-w` imperativo (no `$derived`): ver REGISTRO |
| P5 — Ancla y restauración | ✅ completado | 7 cambios + deriva + 500 saltos; negativa: 6 fallos sin guarda | Guarda por `anchoredLeft` en lugar de `restoring` |
| P6 — Teclado | ✅ completado | unit 46 | Zoom del WebView no medido: confirmación del titular |
| P7 — Ventana de preferencias | ✅ completado | `tauri dev`: abrir/enfocar, `prefs.json`, recorte, Esc, log limpio | `capabilities/prefs.json` (`allow-close`); visuales pendientes del titular |
| P8 — Cierre y commit | ✅ completado | negativa de H4; `pnpm verify` final | |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- Pantalla completa y regla √2, `ForcePreferredFont`, `ForceForegroundColor`, `ForceAlignment`, `ShowFullscreenMessage`, `Doc.lang` (fase 5).
- Ctrl+V, posiciones persistentes, `initial_char_index`, regla 1 del enrutado (fase 3); búsqueda (4); Ctrl+S (7).
- Escala de fuente con Ctrl+rueda; persistir la escala; vista previa en vivo mientras se arrastra un control (se aplica en `change`).
- Lista de fuentes del sistema (sólo `datalist` fijo + texto libre).
- `LoadingOverlay`, `Block.svelte`, `state/*`, `theme/tokens.css`.

**Hallazgos aparcados**
- (vacío)
