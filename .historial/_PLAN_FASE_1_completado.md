# Plan de Implementación: Fase 1 — Texto plano en columnas, enrutado y errores

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes e incuestionables:** fijadas con el titular. Durante auditoría o implementación **queda prohibido cuestionarlas, modificarlas o reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni las decisiones D1–D11 de `ARCHITECTURE.md`.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| G1 | **Entrada de documentos en fase 1: argumento de lanzamiento + Ctrl+O** (`open_dialog` se adelanta de la fase 2). Sólo el primer argumento; varios argumentos, drop, portapapeles y single-instance siguen en fase 3. | Decisión del titular. El argumento ejercita "apertura de lanzamiento fallida → ventana vacía + aviso" (§9.1); Ctrl+O permite probar a mano y ejercita el enrutado a ventana nueva. | P4, P5 |
| G2 | **`ts-rs` 12** genera los tipos TS (no `specta`). Un archivo por tipo en `src/lib/ipc/types/`, generado por `cargo test` y versionado. | `specta`/`tauri-specta` siguen en *release candidate* (2.0.0-rc.28 / rc.26); `ts-rs` es estable. Cierra el *(verificar)* de §5. | P3 |
| G3 | **Las ventanas de lectura las crea Rust**, también `reader-0`: se quita `app.windows` de `tauri.conf.json` y se crean en `setup` **después** de registrar el `Doc` pendiente de lanzamiento. Etiquetas `reader-{n}` con contador atómico. | Elimina la carrera de §1.3 del cierre: el pendiente existe antes de que exista la ventana. | P4 |
| G4 | **Modelo mínimo por fase:** `Doc { title, blocks }`, `Block::Paragraph { runs }`, `Run { text }`. El resto de campos de §5 (`doc_key`, `source`, `is_plain`, `lang`, `initial_char_index`, estilos de `Run`, otras variantes de `Block`) entran con la fase que los usa. | Evita campos y variantes sin consumidor. El contrato crece sin romperse (campos opcionales). | P3 |
| G5 | **Texto plano: un `Paragraph` por línea**, también las vacías. Las líneas vacías conservan su alto con CSS (`min-height: 1lh`). | Fidelidad al original (Tofu respeta los saltos) y el índice cuadra solo: separador de bloque = `\n` (§5). | P3, P5 |
| G6 | **El `pitch` medido en el DOM entra en fase 1** (mínimo: medir y desplazar). Fase 2 conserva el ancho exacto de viewport, el ancla, el reflow y el test de 500 saltos. | Sin `pitch` real no hay "teclado por columnas" correcto: `column-width` es un mínimo (§6). | P6 |
| G7 | **Desplazamiento animado con `scrollTo({ behavior })` nativo** (`'smooth'`, o `'instant'` con `prefers-reduced-motion`), no con `requestAnimationFrame`. Se navega sobre una **columna objetivo** guardada, no sobre el `scrollLeft` en curso. | Menos código; el destino es siempre `round(k × pitch)`, así que no hay error acumulado ni hace falta el ajuste final de §6. | P6 |
| G8 | **Rueda en fase 1** (`deltaY`/`deltaX` → `scrollLeft`, sin ancla). | Sin ella la rueda no hace nada en un lector horizontal; son pocas líneas. | P6 |
| G9 | **`App.svelte` monta `Reader` directamente.** La decisión por *label* llega en fase 2 con la ventana de preferencias. | Con un único tipo de ventana el `if` no tiene consumidor. | P5 |
| G10 | **Un pánico en el importador es `Internal`** en fase 1 (`Corrupt` llega en fase 5, §5 Errores). | Taxonomía por fases ya fijada. | P4 |

## 2. Objetivo

Criterio de hecho de §13 (fase 1): **un `.txt` largo se lee en columnas y se navega con el teclado; un archivo ilegible o no soportado muestra un aviso nativo y queda en el log.** Se construye: importador de texto plano (`Doc`), tipos generados, enrutado con `Doc` pendiente, `AppError` (`Read`, `Unsupported`, `Internal`) + `report()`, log a archivo y panic hook, y el frontend de lectura con CSS multicol, teclado, rueda y Ctrl+O.

## 3. Resumen del análisis

- **Estado heredado (`7777d54`):** `src-tauri/src/lib.rs` = `Builder::default().run(...)`, sin plugins ni comandos; `Cargo.toml` sólo con `tauri 2`; `tauri.conf.json` crea `reader-0` (1000×700, mín. 480×360) y fija la CSP; `capabilities/reader.json` = `core:event:default` + `core:window:default`; `src/App.svelte` = `<main>` vacío; sin `@tauri-apps/api`; `tests/layout/multicol.test.ts` prueba la premisa D1; `package.json` `test` con `--passWithNoTests`. Rama `main`, sólo `_CIERRE_FASE_0.md` sin seguimiento.
- **Versiones verificadas (2026-11-11):** `tauri-plugin-dialog` 2.8.4, `tauri-plugin-log` 2.10.2 (crates.io marca la 3.0 alpha como *latest*: fijar `"2"`), `ts-rs` 12.0.1, `serde` 1.0.229, `serde_json` 1.0.154, `encoding_rs` 0.8.44, `chardetng` 1.0.0, `thiserror` 2.0.21, `log` 0.4.35; npm `@tauri-apps/api` 2.12.4 (*latest*; la 3 es `next`).
- **Patrones existentes:** ninguno de producto. Se siguen ARCHITECTURE §4 (rutas), §5 (contrato, enrutado, errores), §6 (motor), §7 (teclas) y los hallazgos de `_CIERRE_FASE_0.md`.
- **Cobertura de pruebas:** sólo `multicol.test.ts` (layout). Unit TS y Rust vacíos.
- **Riesgos:** (1) crear ventanas desde un comando **síncrono** bloquea en Windows → los comandos que crean ventanas son `async`; (2) llamar a `blocking_pick_file` en el hilo principal bloquea → el diálogo y la importación corren en `spawn_blocking`; (3) perder `document-opened` si se emite antes de `listen()` → el frontend registra `listen` **antes** de `take_pending_document`; (4) `parent` del diálogo de mensaje: la API existe (`MessageDialogBuilder::parent`, `show(cb)` no bloqueante, dialog 2.8.4); su comportamiento se observa en P2; (5) el DOM añadiendo texto entre bloques rompería D9 → test de layout; (6) el `listen()` global de `@tauri-apps/api` escucha con destino `Any` y recibe también los `emit_to` dirigidos a otras ventanas → se escucha sobre la ventana actual; (7) Cargo busca `.cargo/config.toml` desde el directorio actual hacia arriba, no desde `--manifest-path` → el archivo va en la raíz del repo.

- **Auditoría (2026-11-11, agy + OpenCode Big Pickle, sólo lectura):** aplicados sus hallazgos (config de Cargo en la raíz, `listen` por ventana, cadena de altura, `pitch` sin redondear, tope de `k`, `route` sin rama muerta, `next` desde 1, título de `reader-0`, importación de arranque por `run_import`, `tick` vía `$effect`, rutas de *fixtures*, entre otros). Rechazado: resolver rutas relativas probando `current_dir().parent()` (se usan rutas absolutas en las puertas).

## 4. Restricciones innegociables

- Nunca `pnpm build`, `pnpm tauri build` ni instaladores. `pnpm tauri dev` sí. Compilación y puertas **desde PowerShell**.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo (salvo los *fixtures* binarios/UTF-16, que se crean con `[IO.File]::WriteAllBytes`).
- D10: ningún comando acepta rutas del frontend. D11: el frontend nunca muestra errores. Sin permisos nuevos en `capabilities/reader.json`.
- D9: índice en UTF-16 sobre texto normalizado; el DOM no añade texto (ni espacios entre bloques, ni `::before/::after` con contenido).
- D8: unidades en px. Sin `pt`.
- Dependencias: sólo las de §3. Nada de `@testing-library/*`.
- Textos visibles provisionales en español (diferido de idioma abierto).

## 5. Checklist previo a los pasos

- [x] Estructura del proyecto revisada
- [x] Archivos afectados leídos (`lib.rs`, `main.rs`, `Cargo.toml`, `tauri.conf.json`, `reader.json`, `App.svelte`, `main.ts`, `package.json`, `vitest.config.ts`, `tsconfig.json`)
- [x] Patrones: ARCHITECTURE como estructura destino
- [x] Decisiones fijadas (G1 con el titular; G2–G10 técnicas, justificadas)
- [x] Cobertura de pruebas mapeada
- [x] Versiones verificadas (§3)
- [x] Consumidores: ninguno (código nuevo)

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- `git status` (sólo `_CIERRE_FASE_0.md` y este plan sin seguimiento) y `pnpm verify` en verde. El punto de vuelta atrás es `7777d54`.

**Gate:** `pnpm verify` sale con 0 sobre `7777d54`.

### Paso 2 — Errores, log y panic hook (Rust)
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/error.rs`
- Dependencias: `tauri-plugin-dialog = "2"`, `tauri-plugin-log = "2"`, `log = "0.4"`, `thiserror = "2"`, `serde = { version = "1", features = ["derive"] }`.
- `error.rs`:
  - `enum AppError { Read { file: String, source: io::Error }, Unsupported { file: String }, Internal(String) }` con `thiserror`. `file` es el **nombre** del archivo (sin ruta); la ruta completa sólo va al log en `report()`.
  - `user_message(&self) -> String`: único `match` con texto visible. `Read` → `No se pudo leer «{file}»: {source}`; `Unsupported` → `«{file}» no es un archivo de texto.`; `Internal` → texto genérico (`Se produjo un error interno.`), el detalle sólo al log.
  - `Serialize` manual a `{ kind, message }` (`kind` = `"read" | "unsupported" | "internal"`), para que los comandos devuelvan `Result<_, AppError>`.
  - `pub fn report(app: &AppHandle, origin: Option<&WebviewWindow>, err: &AppError, detail: &str)`: `log::error!("{detail}: {err:?}")` y diálogo **no bloqueante**: `app.dialog().message(err.user_message()).kind(MessageDialogKind::Error).title("Miso")`, `.parent(w)` si hay `origin`, `.show(|_| {})`. `detail` lleva la ruta completa cuando la hay.
- `lib.rs`: registrar `tauri_plugin_log::Builder::new()` con `clear_targets()` (por defecto trae Stdout + LogDir), `target(Target::new(TargetKind::LogDir { file_name: None }))`, `max_file_size(1_000_000)`, `RotationStrategy::KeepOne`, nivel `Info`; añadir `TargetKind::Stdout` sólo con `cfg!(debug_assertions)`. Después `tauri_plugin_dialog::init()`. En `setup`, `std::panic::set_hook` que registra `log::error!` con mensaje y ubicación y luego llama al hook previo (`take_hook`).
- Tests en `error.rs`: `user_message` no contiene separadores de ruta; serialización de cada variante a `{kind, message}`.

**Gate:** `pnpm lint:rust` y `pnpm test:rust` en verde. `pnpm tauri dev`: existe `%LOCALAPPDATA%\com.provisional.miso\logs\Miso.log` o `miso.log` (anotar el real en la tabla) con al menos una línea. Verificación temporal (revertida): un `report(...)` forzado en `setup` muestra el diálogo de error **sobre** la ventana y la app sigue respondiendo mientras está abierto; un `panic!` forzado en un hilo deja su línea en el log.

### Paso 3 — Modelo, tipos generados e importador de texto plano (Rust)
**Archivos:** `src-tauri/Cargo.toml`, `.cargo/config.toml` (raíz del repo), `src-tauri/src/model.rs`, `src-tauri/src/import/mod.rs`, `src-tauri/src/import/plain.rs`, `src-tauri/src/lib.rs`, `tests/fixtures/*`, `src/lib/ipc/types/*` (generados), `package.json`
- Dependencias: `ts-rs = "12"`, `encoding_rs = "0.8"`, `chardetng = "1"`; `serde_json = "1"` en `[dev-dependencies]`.
- `.cargo/config.toml` **en la raíz del repo** (riesgo 7; también lo encuentra `tauri dev`, que corre en `src-tauri/`): `[env] TS_RS_EXPORT_DIR = { value = "src/lib/ipc/types", relative = true }` (`relative` resuelve contra el padre de `.cargo/`).
- `model.rs` (G4): `Doc { title: String, blocks: Vec<Block> }`; `#[serde(tag = "type")] enum Block { Paragraph { runs: Vec<Run> } }`; `Run { text: String }`. `#[derive(Serialize, Clone, Debug, TS)] #[ts(export)]`.
- `import/mod.rs`: `pub fn import(path: &Path) -> Result<Doc, AppError>`: lee con `std::fs::read` (error → `Read`), `title` = nombre del archivo, delega en `plain::parse(bytes)`. Es el punto de entrada único que ampliarán las fases 5–6 (sniff por contenido, §9.5).
- `import/plain.rs`, `parse(&[u8]) -> Result<Vec<Block>, Unsupported>`:
  1. BOM (UTF-8, UTF-16LE, UTF-16BE) → `encoding_rs::Encoding::for_bom` y decodificar.
  2. Sin BOM: si hay `0x00` en los primeros 8 KB → `Unsupported` (binario).
  3. UTF-8 válido → UTF-8; si no, `chardetng::EncodingDetector` (`guess(None, true)`) y decodificar.
  4. Normalizar `\r\n` y `\r` → `\n`; eliminar `U+FFFC` (§5).
  5. `split('\n')` → un `Paragraph` con un `Run` por línea (G5). Archivo vacío → `blocks` vacío (no es error, §5).
- `fn flat_len(doc: &Doc) -> usize` (sólo `#[cfg(test)]`): suma `encode_utf16().count()` de los runs + `blocks.len().saturating_sub(1)`.
- *Fixtures* en `tests/fixtures/`: `utf8.txt`, `utf8-bom.txt`, `utf16le.txt`, `utf16be.txt`, `cp1252.txt` (con `ñ`, `é`, `€`), `binario.bin` (con `0x00`), `vacio.txt`, `unicode.txt` (CRLF, CR, emoji con par sustituto, CJK, `e` + U+0301, `U+FFFC`, línea vacía, salto final). Los no-UTF-8 se crean con `[IO.File]::WriteAllBytes`. `unicode.doc.json` = salida serializada del importador para `unicode.txt`, versionada.
- Los tests localizan los *fixtures* con `concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/…")`.
- Tests: cada *fixture* decodifica al texto esperado; `vacio.txt` → `blocks` vacío y `flat_len` 0; `binario.bin` → `Unsupported`; ruta inexistente → `Read`; `unicode.txt` → sin `\r` ni `U+FFFC` y `flat_len` = valor fijo **K** escrito a mano en el test; el `Doc` coincide con `unicode.doc.json`.
- `package.json`: `verify` pasa a `pnpm lint:rust && pnpm test:rust && pnpm check && pnpm test && pnpm test:layout` (los tipos se regeneran antes de comprobarlos).

**Gate:** `pnpm test:rust` en verde y crea `src/lib/ipc/types/{Doc,Block,Run}.ts`; una segunda ejecución no cambia nada (`git status` sin diferencias en `src/lib/ipc/types/`).

### Paso 4 — Enrutado, ventanas y comandos (Rust)
**Archivos:** `src-tauri/tauri.conf.json`, `src-tauri/src/doc_windows.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
- `tauri.conf.json`: `app.windows: []` (G3).
- `doc_windows.rs`:
  - Estado puro y testeable: `struct Slots(HashMap<String, Slot>)`, `enum Slot { Pending(Doc), Open(Doc) }`. Ventana ausente del mapa = vacía. Operaciones: `route(origin: &str, doc, new_label) -> Route { Deliver(label) | NewWindow(label) }` (origen ausente del mapa → `Deliver` y queda `Open`; origen `Open` o `Pending` → `NewWindow(new_label)` y queda `Pending`); `take(label) -> Option<Doc>` (`Pending` → `Open`, devuelve el `Doc`; en otro caso `None`); `close(label)`. Reglas 2 y 3 de §5; la 1 (mismo `doc_key`) es fase 3.
  - `DocWindows { slots: Mutex<Slots>, next: AtomicU32 }` como `State`; `next` arranca en **1** (`reader-0` es la de arranque).
  - `create_reader(app, label, title) -> Result<WebviewWindow, AppError>`: `WebviewWindowBuilder::new(app, label, WebviewUrl::default())`, título `title`, 1000×700, mínimo 480×360. Fallo → `Internal`.
  - `async fn run_import(path) -> Result<Doc, AppError>`: `spawn_blocking(import)`; `JoinError` (pánico) → `Internal` (G10). Es el único camino de importación (arranque y Ctrl+O).
  - `async fn open_path(app, origin: WebviewWindow, path)`: `run_import`. Con `Ok(doc)`: `route`; `Deliver` → `emit_to(label, "document-opened", &doc)` y `set_title(&doc.title)`; `NewWindow` → `create_reader(.., &doc.title)`. Con `Err` → `report` con la ventana de origen.
  - En `on_window_event`, `WindowEvent::Destroyed` → `close(label)`.
- `commands.rs`, ambos `async` (riesgo 1):
  - `take_pending_document(window: WebviewWindow) -> Option<Doc>`: usa la etiqueta de la ventana que llama (no acepta etiqueta del frontend).
  - `open_dialog(app, window)`: en `spawn_blocking`, `app.dialog().file().set_parent(&window).add_filter("Texto", &["txt"])` + filtro "Todos" (`*`) y `blocking_pick_file()` → `Option<FilePath>`; cancelar → nada (§5). Con ruta → `into_path()` y `open_path(app, window, path).await`.
- `lib.rs`: los comandos se registran en el `Builder` con `.invoke_handler(tauri::generate_handler![take_pending_document, open_dialog])`; `on_window_event` como arriba. En `setup`: `manage(DocWindows)`; primer argumento de `std::env::args_os().skip(1)` que no empiece por `-`, relativo resuelto contra `current_dir()`. Con argumento: `block_on(run_import(path))` **antes** de crear la ventana; `Ok(doc)` → `Pending` en `reader-0` y `create_reader("reader-0", &doc.title)`; `Err` (incluido pánico) → `create_reader("reader-0", "Miso")` y `report` sobre ella (§9.1). Sin argumento: `reader-0` vacía.
- Tests sobre `Slots`: vacía + doc → `Deliver`; con doc (`Open` o `Pending`) → `NewWindow` y `take` lo devuelve una sola vez; `close` libera, también con un pendiente sin recoger. Sobre la conversión de `JoinError`: una tarea `spawn_blocking` que entra en pánico → `Internal`.

**Gate:** `pnpm lint:rust` y `pnpm test:rust` en verde. Con **rutas absolutas** (en `tauri dev` el directorio actual es `src-tauri/`): `pnpm tauri dev -- -- E:\no\existe.txt` → ventana vacía con aviso `Read` encima y línea en el log con la ruta completa; con `E:\Codigo\Miso\tests\fixtures\binario.bin` → aviso `Unsupported`. (El frontend aún no pinta: la recepción se comprueba en P5.)

### Paso 5 — Frontend: IPC, índice y columnas
**Archivos:** `package.json`, `src/lib/ipc/commands.ts`, `src/lib/ipc/events.ts`, `src/lib/layout/textIndex.ts`, `src/lib/layout/textIndex.test.ts`, `src/components/Reader.svelte`, `src/components/Columns.svelte`, `src/components/EmptyState.svelte`, `src/App.svelte`, `vitest.config.ts`, `tests/layout/columns.test.ts`
- `pnpm add @tauri-apps/api@^2.12.4` (dependencia de producción). Quitar `--passWithNoTests` de `test`.
- `ipc/commands.ts`: `takePendingDocument(): Promise<Doc | null>`, `openDialog(): Promise<void>` (Rust presenta los errores, D11). `ipc/events.ts`: `onDocumentOpened(cb): Promise<UnlistenFn>` con `getCurrentWebviewWindow().listen('document-opened', cb)`, **nunca** el `listen` global (riesgo 6).
- `layout/textIndex.ts`: `blockStarts(doc): number[]` y `flatLength(doc): number` con `String.length` (UTF-16, D9) y separador 1.
- `textIndex.test.ts`: con `tests/fixtures/unicode.doc.json`, `flatLength` = el mismo **K** del test de Rust (paridad).
- `App.svelte` → `<Reader />` (G9), conservando la cadena de altura: `:global(html), :global(body), :global(#app) { margin: 0; height: 100%; }` (sin ella el multicol no desborda). `Reader.svelte`: `$state` con el `Doc`; al montar, **primero** `onDocumentOpened`, **después** `takePendingDocument` (riesgo 3); limpia el `listen` al desmontar. Sin `Doc` → `EmptyState` ("Abre un archivo con Ctrl+O"); con `Doc` → `Columns`.
- `Columns.svelte`: `.viewport` (`overflow-x: auto; overflow-y: hidden; height: 100%`) > `.columns` (`height: 100%; box-sizing: border-box; column-width: 360px; column-gap: 21.6px; column-fill: auto; padding: 20px 10.8px;` fuente `"Palatino Linotype", Palatino, serif` 14px, `line-height: 1.1`, `text-align: justify`) > un `<p data-char-start>` por bloque con `margin: 0; min-height: 1lh`. Valores por defecto de §6/§8 (inset 3 % de 360 = 10.8 px, gap = 2 × inset). Sin espacios en blanco entre elementos en la plantilla que generen nodos de texto.
- `vitest.config.ts`: añadir `plugins: [svelte()]` al proyecto `layout`.
- `tests/layout/columns.test.ts`: monta `Columns` con `unicode.doc.json` (`mount` de `svelte`, `unmount` en `finally`) dentro de un contenedor de 1000×700 px: cada `<p>` contiene sólo el texto de sus runs, cada `data-char-start` coincide con `blockStarts`, y `último data-char-start + longitud del último <p>` = **K** (los separadores no están en el DOM: los aporta el índice).

**Gate:** `pnpm verify` en verde (unit con tests reales). `pnpm tauri dev -- -- <txt largo>` (≥ 200 KB, generado en el *scratchpad*) muestra el texto en columnas con desbordamiento horizontal; Ctrl+O aún no (P6).

### Paso 6 — Frontend: navegación por columnas, rueda y Ctrl+O
**Archivos:** `src/lib/nav/columnScroll.ts`, `src/lib/nav/keyboard.ts`, `src/lib/nav/keyboard.test.ts`, `src/lib/nav/wheel.ts`, `src/components/Columns.svelte`, `src/components/Reader.svelte`, `tests/layout/columns.test.ts`
- `columnScroll.ts` (G6, G7):
  - `measurePitch(columns: HTMLElement): number | null`: recorre los nodos de texto con `Range.getClientRects()` hasta encontrar dos `left` que difieran en más de 1 px y devuelve su diferencia **sin redondear** (redondear acumula error por columna); `null` si todo cabe en una columna.
  - `createColumnScroller(viewport, columns)`: guarda `pitch` y la columna objetivo `k`; `go(delta)`, `goTo(k)`, `first()`, `last()`. Con `maxScroll = scrollWidth − clientWidth`: `kMax = floor((maxScroll + 1) / pitch)`; destino `min(round(k × pitch), maxScroll)`; `last()` = `maxScroll`. `scrollTo({ left, behavior })` según `prefers-reduced-motion`. `visibleColumns() = max(1, floor((clientWidth + gap) / pitch))`. Tras un scroll ajeno (rueda), en `scrollend`: `k = round(scrollLeft / pitch)` (o `kMax` si `scrollLeft ≥ maxScroll − 1`). Con `pitch` `null` (cabe en una columna) todo es no-op. `remeasure()` en `ResizeObserver` del viewport y en un `$effect` de `Columns` sobre el `Doc` (se ejecuta tras actualizar el DOM): vuelve a medir y ajusta al inicio de columna más cercano (sin ancla: fase 2).
  - `Columns.svelte` expone `go`, `goTo`, `first`, `last` y `visibleColumns` con `export function`; `Reader` los usa vía `bind:this`.
- `keyboard.ts`: función pura `actionFor(e: { key, ctrlKey, shiftKey, altKey, metaKey }): Action | null`: `ArrowLeft/ArrowRight` → ±1 columna; `PageUp/PageDown` y `' '`/`Mayús+' '` (`e.key === ' '`) → ± `visibleColumns`; `Home/End` → primera/última; `Ctrl+O` → abrir. Sin modificadores ajenos. Flechas ↑/↓, Ctrl+V y el resto de §7.1 quedan fuera (§9).
- `keyboard.test.ts`: tabla tecla → acción, incluido que `Ctrl+ArrowRight` y `Alt+Home` no hacen nada.
- `wheel.ts` (G8): en `wheel` no pasivo del viewport, si `|deltaY| > |deltaX|` → `scrollLeft += deltaY` (con `deltaMode` de línea × 16) y `preventDefault`; `deltaX` se deja al navegador.
- `Reader.svelte`: `keydown` en `window` → `actionFor` → `preventDefault` y scroller u `openDialog()`.
- `columns.test.ts`: con texto largo en un contenedor de 1000×700 px, `measurePitch` coincide con la diferencia de `left` de dos columnas consecutivas; tras `goTo(k)` con `behavior: 'instant'` para k = 1..20, el primer rect visible empieza a < 1 px del borde izquierdo más el padding.

**Gate:** `pnpm verify` en verde. Manual en `pnpm tauri dev` con el txt largo: → / ← mueven una columna exacta; Av Pág y Espacio avanzan lo visible; Inicio/Fin van a los extremos; la rueda desplaza en horizontal; redimensionar deja una columna alineada al borde. Ctrl+O con la ventana con documento abre **otra** ventana con el elegido; Ctrl+O en una ventana vacía lo carga en ella **y ninguna otra ventana cambia de documento**; cancelar no hace nada.

### Paso 7 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, este plan, `_CIERRE_FASE_0.md`
- Pruebas negativas (revertidas): un espacio literal entre bloques en `Columns.svelte` hace fallar `columns.test.ts`; cambiar **K** en el test de Rust hace fallar `test:rust`.
- `ARCHITECTURE.md`: §4 (`.cargo/config.toml` en la raíz, `ipc/types/` en lugar de `types.ts`, sin `windows` en la conf); §5 Eventos (escuchar con la ventana actual, no con `listen` global); §5 (`ts-rs` fijado; modelo por fases; `open_dialog` en fase 1); §6 (`scrollTo` nativo con columna objetivo); §13 (fase 1 incluye Ctrl+O y rueda; fase 2 quita lo ya hecho).
- `DIFERIDOS.md`: lo aparcado en §9 que siga abierto, con fecha y hora.
- `REGISTRO.md`: asiento de fase 1.
- Renombrar a `_PLAN_FASE_1_completado.md` → `.historial/`. `_CIERRE_FASE_0.md` → `.historial/.informes/_CIERRE_FASE_0_cumplido.md`.
- Commit `feat: fase 1 — texto plano en columnas, enrutado y errores`.

**Gate:** `pnpm verify` verde sobre el commit final; `git status` limpio; ni el plan ni el cierre siguen en la raíz.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5 ─→ P6 ─→ P7
```
P3 usa `AppError` (P2); P4 usa `import` (P3); P5 necesita los tipos (P3) y los comandos (P4); P6 necesita `Columns` (P5).

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | 2026-11-11: `pnpm verify` = 0 sobre `7777d54` | |
| P2 — Errores, log y panic hook | ✅ completado | 2026-11-11: clippy y `cargo test` (3/3) en verde; `tauri dev` crea `logs\Miso.log`; pánico forzado en hilo registrado con ubicación (revertido); la ventana sigue abierta | desviaciones: `serde_json` en dev-deps desde P2 (test de serialización); `#[allow(dead_code)]` puntual hasta P4; línea `info` de arranque añadida. Diálogo sobre la ventana: se observa en P4 con errores reales |
| P3 — Modelo, tipos e importador | ✅ completado | 2026-11-11: `pnpm verify` = 0 (16 tests Rust); tipos en `src/lib/ipc/types/` estables entre ejecuciones; sin `src-tauri/bindings/`; K = 25 | desviaciones: API real de chardetng 1.0 (`Iso2022JpDetection::Deny`, `Utf8Detection::Allow`); `tests/fixtures/.gitattributes` `* -text` para conservar CRLF/CR en git |
| P4 — Enrutado, ventanas y comandos | ✅ completado | 2026-11-11: clippy y `cargo test` (21) en verde; `tauri dev` con ruta inexistente → ventana "Miso" + `Read` en el log con ruta completa; con `binario.bin` → `Unsupported`; con `utf8.txt` → ventana titulada `utf8.txt` | desviaciones: `Slot::Open` sin `Doc` (sin consumidor hasta fase 3/7); `Slots::park` para el pendiente de arranque. El diálogo sobre la ventana no se comprobó visualmente (regla de no verificación visual): lo confirma el titular |
| P5 — IPC, índice y columnas | ✅ completado | 2026-11-11: `pnpm verify` = 0 (Rust 21, check 0/0, unit 3/3, layout 2/2) | desviación: Svelte deja un nodo de texto vacío como ancla de `{#each}`; el test exige que ningún nodo de texto entre bloques tenga contenido. La vista en columnas con el txt largo se comprueba en P6 |
| P6 — Navegación, rueda y Ctrl+O | ✅ completado | 2026-11-11: `pnpm verify` = 0 (unit 21, layout 4: `pitch` ±0.5 px y 20 saltos alineados < 1 px); `tauri dev` con txt de 213 KB → ventana `largo.txt`, sin errores en el log | desviaciones: `goTo(k, behavior?)` para forzar `'instant'` en el test; `sync` expuesto para `scrollend`. Pruebas manuales de teclado, rueda, redimensionado y Ctrl+O: pendientes del titular (sin verificación visual) |
| P7 — Cierre documental y commit | ✅ completado | 2026-11-11: negativas (espacio literal entre bloques → layout falla; K = 26 → `test:rust` sale con 101), revertidas; `pnpm verify` verde | la primera negativa con un espacio suelto no falló: Svelte 5 elimina el espacio final de `{#each}`; se repitió con `{' '}` |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- Ancla, reflow con restauración, ancho de viewport múltiplo exacto, test de 500 saltos, ↑/↓, prefs, escala de fuente, atajos de §7.1 salvo Ctrl+O, `hyphens` (fase 2).
- Regla 1 del enrutado (mismo `doc_key`), posiciones, portapapeles, drop, varios argumentos, single-instance, "y N más" (fase 3).
- Búsqueda (4), HTML/DOCX/fullscreen (5), RTF/PDF y `Corrupt` (5–6), exportar y asociaciones de archivo (7), rendimiento y segmentación (8).
- `LoadingOverlay`, `Block.svelte`, `state/*.svelte.ts`, `theme/tokens.css`: llegan cuando tengan consumidor.
- Límite de tamaño de archivo.

**Hallazgos aparcados**
- 2026-11-11 — Los comandos propios quedan accesibles a cualquier ventana (sin `AppManifest`). Con una sola clase de ventana es inocuo; revisar en fase 2 al añadir `prefs`.
