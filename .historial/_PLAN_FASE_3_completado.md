# Plan de Implementación: Fase 3 — Posición persistente, portapapeles, drop y una sola instancia

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> H2, H5 y H7 cambian lo escrito en ARCHITECTURE §5/§8. El titular delegó la decisión en el orquestador (2026-11-12), que las mantiene por ser las de menor código.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| H1 | **Toda entrega a una ventana pasa por el pendiente.** `Route::Deliver` deja el `Doc` como pendiente de la ventana de origen y le emite `document-ready` (sin carga); el frontend responde con `take_pending_document`, igual que al montar. | Elimina la carrera evento/`listen()` en cualquier momento (también en el arranque) y permite un único `open_paths` para arranque, Ctrl+O, drop y segunda instancia. `take` devuelve el `Doc` una sola vez, así que recogerlo por las dos vías es inocuo. | P4, P8 |
| H2 | **El frontend no conoce la clave del documento.** `set_position(char_index)`: Rust resuelve la clave por la ventana que llama. `Doc` sólo gana `initial_char_index: u32` (0 = inicio); `doc_key` sale del modelo de §5. | D10 en espíritu (el frontend no maneja identidades de archivo) y un campo menos en el contrato. | P2, P5, P8 |
| H3 | **`DocKey { file_id: Option<String>, content_hash: u64, path: String }`.** `file_id` = número de serie del volumen + ID de 128 bits (`GetFileInformationByHandleEx` + `FileIdInfo`, `windows-rs`) como `"{vol:016x}-{:032x}"` con `u128::from_le_bytes(Identifier)`; cualquier fallo → `None`. `content_hash` = FNV-1a de 64 bits sobre el tamaño en `u64` LE y después los primeros 64 KB (en streaming, sin copiar), escrito a mano. `path` = `fs::canonicalize` (si falla, la ruta recibida) pasada a `String` con `to_string_lossy`. | El `DefaultHasher` de std no es estable entre versiones de Rust y la clave se persiste. FNV son cinco líneas sin dependencia; las colisiones las desempata `path`. `String` evita que una ruta no UTF-8 impida serializar todo `positions.json`. | P3 |
| H4 | **Mismo archivo** (regla 1 del enrutado) = misma `path` **o** ambos `file_id` presentes e iguales. El hash no cuenta. **Búsqueda de posición** (§8): `file_id` → `content_hash` (si varias, la de igual `path`; si ninguna, la más reciente) → `path`. | Dos copias idénticas son dos documentos al abrir, pero comparten posición si una se movió de disco (que es lo que pide §8). La `path` cubre un `file_id` inestable (exFAT, red). | P4, P5 |
| H5 | **`positions.json` = `{ schema_version: 1, entries: [{ file_id?, content_hash, path, char_index, date_changed }] }`**, con `Entry { #[serde(flatten)] key: DocKey, char_index: u32, date_changed: u64 }` (milisegundos desde epoch, UTC). Otra versión → se trata como corrupto (`*.corrupt` y vacío). Tope de **1000** entradas; al guardar, `prune(&mut self)` quita las de `date_changed` más antiguo. Sin migraciones hasta que exista la versión 2. | Tofu no poda; un tope evita crecer sin límite con una línea. Migrar sin una versión anterior es infraestructura sin consumidor. | P5 |
| H6 | **Debounce de 5 s con un hilo `std::thread`** (sin `tokio` directo) y **un solo `Mutex<Inner { positions, dirty, scheduled }>`**: el primer `set_position` tras un guardado programa uno a los 5 s; flush inmediato al destruirse una ventana lectora y en `RunEvent::Exit`. `flush` guarda **dentro** del cerrojo (serializa los tres invocadores y el `.tmp`) y sólo apaga `dirty` si el guardado fue bien. Fallo de escritura → `Persist` sólo al log (§5). | Igual que `_saveTimer` de Tofu. Un cerrojo en vez de dos atómicos: no se pierde el último cambio si falla el disco ni se pisan dos guardados. | P5 |
| H7 | **Portapapeles con `arboard`.** `ContentNotAvailable` → nada; ocupado u otro error → sólo log; texto vacío o sólo blancos → nada. Documento titulado **«Portapapeles»**, sin clave ni posición. Al arrancar sin rutas se abre si hay texto (§9.3). Ctrl+V con el foco en un campo del panel de preferencias pega en el campo. | `arboard` distingue exactamente los casos de §5. Escribir el acceso al portapapeles con `windows-rs` serían ~30 líneas `unsafe` para lo mismo. | P6, P8 |
| H8 | **Drop:** sólo archivos; un directorio se ignora con `log::info!`. Varios archivos → en el orden recibido, como una tanda. | §9.2. Un directorio daría un «Acceso denegado» engañoso. | P4 |
| H9 | **Segunda instancia** (`tauri-plugin-single-instance`): con rutas → `open_paths` con origen = **última ventana lectora enfocada** (`WindowEvent::Focused(true)`; si ya no existe, cualquiera); sin rutas → enfoca esa ventana. Las rutas relativas se resuelven contra el `cwd` que entrega el plugin. | §9.1. Tauri no expone "última enfocada"; un `Mutex<String>` basta. | P7 |
| H10 | **Errores de una tanda:** un diálogo con `user_message` del primero y, si hay más, `"\n\nY {n} más."`; todos al log. Función pura `batch_message`. | §5, "Varios fallos de una tanda". | P4 |
| H11 | **Dependencias nuevas (sólo estas):** `tauri-plugin-single-instance = "2"`, `arboard = { version = "3", default-features = false }` (sin `image-data`; requiere red en la primera compilación), `windows` con la versión ya presente en `Cargo.lock` (0.62) y features `Win32_Foundation`, `Win32_Storage_FileSystem`. | Mínimo para H3, H7 y H9. Misma versión de `windows` que Tauri: no se compila dos veces. | P3, P6, P7 |

## 2. Objetivo

Criterio de hecho de §13 (fase 3): **reabrir un archivo vuelve al punto exacto.** Se construye: posición persistente por documento, Ctrl+V y portapapeles al arrancar, drag & drop de archivos, regla 1 del enrutado (archivo ya abierto → se enfoca su ventana) y una sola instancia.

## 3. Resumen del análisis

- **Estado heredado (`18e755f`):** `Slots` puro (`Pending(Doc)` | `Open`, sin clave); `open_path` = importar + `route` + `emit_to("document-opened", doc)` o `create_reader`; `ventana_inicial` importa **sólo el primer argumento** con `block_on` dentro de `setup` (el bucle de eventos aún no sirve IPC, así que el pendiente siempre se recoge); `prefs.rs` con `load`/`save` atómicos y `*.corrupt`; `Columns`/`columnScroll` con `anchor` interno que arranca en 0 y `sync` que sólo lo actualiza en `scrollend` de usuario; `Reader` deja pasar los atajos con Ctrl dentro del panel (`inPanel && !e.ctrlKey`).
- **Pendiente heredado:** confirmación del titular de la fase 2 (`DIFERIDOS.md`, "Antes de la fase 3"). Bloquea P2.
- **Riesgos:**
  1. Entregar por evento a una ventana recién creada pierde el `Doc` (§5): resuelto por H1.
  2. `Reader` con Ctrl+V dentro del panel abriría el portapapeles en vez de pegar (`inPanel && !e.ctrlKey` deja pasar los atajos con Ctrl): H7, condición nueva en P8 y comprobación del titular en P9.
  3. El repositorio está en exFAT: los tests de `file_id` usan `std::env::temp_dir()` (NTFS); si allí da `None`, el test falla indicando el volumen.
  4. `restore()` inicial corre en `onMount`; con fuente del sistema (Palatino Linotype) el layout es síncrono. Si la fuente cambiara tras montar, el `ResizeObserver` no salta (no cambia el tamaño del viewport): se comprueba en P9 con un archivo largo.
  5. `set_focus` desde la instancia principal puede quedar en parpadeo de la barra de tareas (bloqueo de primer plano de Windows). Se comprueba en P9; si sólo parpadea, se acepta.
  6. Posición desactualizada si la app muere sin `Exit`: se pierden como mucho 5 s (como Tofu).
  7. `open_paths` dentro de `setup` vía `block_on` crea ventanas extra antes de que arranque el bucle: ya ocurre hoy con `reader-0`; se comprueba en P9 con dos argumentos.
  8. La ventana de origen puede cerrarse mientras se importa: `route` vería su *label* libre y aparcaría el `Doc` en una ventana inexistente, sin aviso (`emit_to` no falla). `open_doc` resuelve la ventana al entrar; si ya no existe, abre una nueva.
  9. El callback de `single-instance` corre dentro de `WM_COPYDATA` (FFI): un pánico allí aborta el proceso. Nada de indexar `argv` a mano.

## 4. Restricciones innegociables

- Nunca `pnpm build`, `pnpm tauri build` ni instaladores. `pnpm tauri dev` sí. Compilación y puertas **desde PowerShell**.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D8, D9 (`initial_char_index` en UTF-16), D10 (ningún comando recibe rutas ni claves), D11 (el frontend no muestra errores).
- Sólo las dependencias de H11.
- Escuchar eventos con `getCurrentWebviewWindow().listen`. Comandos que crean ventanas, `async`. Diálogos `blocking_*` dentro de `spawn_blocking`.

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos (`lib.rs`, `commands.rs`, `doc_windows.rs`, `error.rs`, `model.rs`, `prefs.rs`, `import/*`, `Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `reader.json`, `Reader.svelte`, `Columns.svelte`, `columnScroll.ts`, `keyboard.ts`, `ipc/*`)
- [x] Evidencia de Tofu (§9.2 y §11 de `rea-tofu-evidence.md`): debounce 5 s, sin poda, `closeSingleUnusedWindow`
- [x] H2, H5 y H7 decididas (delegadas por el titular, 2026-11-12)
- [x] Confirmación del titular de la fase 2 (2026-11-12); Ctrl+rueda añadido aparte
- [x] Cobertura actual: Rust 33, unit 46+, layout 15

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- Confirmaciones pendientes de la fase 2 (`DIFERIDOS.md`).
- `git status` limpio salvo este plan; `pnpm verify` en verde. Punto de vuelta atrás: `18e755f`.

**Gate:** `pnpm verify` = 0 sobre `18e755f`.

### Paso 2 — Persistencia común, modelo y texto plano reutilizable
**Archivos:** `src-tauri/src/store.rs` (nuevo), `src-tauri/src/lib.rs` (`mod store;`), `src-tauri/src/prefs.rs`, `src-tauri/src/model.rs`, `src-tauri/src/import/{mod,plain}.rs`, `src-tauri/src/doc_windows.rs` (helper `doc()` de tests), `src/lib/ipc/types/Doc.ts` (generado), `tests/fixtures/unicode.doc.json`, `src/lib/layout/textIndex.test.ts`, `tests/layout/defaults.ts` (literales de `Doc`)
- `store.rs`: `load<T: DeserializeOwned + Default>(path, ok: impl Fn(&T) -> bool) -> T` y `save<T: Serialize>(path, &T) -> io::Result<()>`, movidos desde `prefs.rs` sin cambiar su comportamiento (inexistente → default; ilegible o `!ok` → default + `*.corrupt` + `log::warn!`; temp + rename). `prefs.rs` los usa (`ok` = siempre) y conserva `validated`.
- `model.rs`: `Doc { title, initial_char_index: u32, blocks }`; `impl Doc { fn flat_len(&self) -> u32 }` (UTF-16 + separadores; sale del test de `import`).
- `plain.rs`: `pub fn blocks(text: &str) -> Vec<Block>` (normalización + partición, hoy dentro de `parse`); `parse` = `decode` + `blocks`.
- `import/mod.rs`: se separan `read(path) -> Result<Vec<u8>, AppError>` y `from_bytes(file, &bytes) -> Result<Doc, AppError>`, para que P4 calcule la clave sobre los mismos bytes sin leer dos veces; `import` queda como envoltorio de dos líneas y sus tests no cambian.
- Tests: los de `prefs` siguen verdes; `import(unicode.txt).flat_len()` = 25 (sustituye al `flat_len` local del test); `blocks("a\r\nb\rc\u{FFFC}")` = 3 párrafos.

**Gate:** `pnpm lint:rust`, `pnpm test:rust` verdes; `Doc.ts` con `initial_char_index: number`; `pnpm verify` verde (literales de `Doc` en Rust y TS con `initial_char_index: 0`).

### Paso 3 — Identidad de archivo
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` (`mod file_id;`), `src-tauri/src/file_id.rs` (nuevo)
- `windows` (H11). `fn file_id(path) -> Option<String>`: abre con `File::open`, `GetFileInformationByHandleEx(handle, FileIdInfo, …)` sobre `FILE_ID_INFO`; formato de H3.
- `fn fnv1a(seed, bytes) -> u64` (encadenable); `fn content_hash(bytes) -> u64` = tamaño LE y después `bytes[..min(len, 64 KB)]`, sin copiar.
- `pub struct DocKey { file_id, content_hash, path }` (`Clone`, `Serialize`, `Deserialize`); `DocKey::new(path, &bytes) -> DocKey` (nunca falla: H3); `fn same_file(&self, other) -> bool` (H4).
- Tests (en un subdirectorio único de `temp_dir()`): `fnv1a` de `b""` = `0xcbf29ce484222325` y de `b"a"` = `0xaf63dc4c8601ec8c`; renombrar conserva `file_id`; dos archivos distintos dan `file_id` distintos; el hash sólo mira tamaño + 64 KB; `same_file`: `path` igual y `file_id` distinto → `true`; `file_id` igual y `path` distinta → `true`; `path` distinta y un `file_id` `None` → `false`. Si `file_id` es `None` en el temporal, el test falla indicando la ruta del volumen.

**Gate:** `pnpm lint:rust`, `pnpm test:rust` verdes; cierra el diferido "Tests de `file_id`".

### Paso 4 — Enrutado: clave, regla 1, tanda y drop
**Archivos:** `src-tauri/src/doc_windows.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/error.rs`, `src/lib/ipc/events.ts`, `src/components/Reader.svelte`
- `Slots`: `Pending(Doc, Option<DocKey>)` | `Open(Option<DocKey>)`; `take` pasa de `Pending(doc, key)` a `Open(key)` **conservando la clave**. `route(origin: Option<&str>, doc, key, new_label) -> Route` con `Route::Focus(label)` si algún slot tiene `same_file` (H4); origen `None` (ventana ya cerrada) → `NewWindow`. `key(label) -> Option<DocKey>` (clonada: no se sostiene el cerrojo de `Slots` mientras se toma el de posiciones). `Deliver` deja el `Doc` como pendiente de `origin` (H1).
- `pub async fn open_paths(app, origin: String, paths: Vec<PathBuf>)`: por cada ruta (H8: directorios fuera), en `spawn_blocking`: `read` → `DocKey::new` → `from_bytes`; después `open_doc`. Errores acumulados → `report_batch` sobre la ventana de origen (si sigue viva). `open_path` desaparece; `open_dialog` llama a `open_paths(origin, vec![path])`.
- `pub fn open_doc(app, origin: &str, doc, key) -> Result<(), AppError>`: resuelve `app.get_webview_window(origin)` una vez (riesgo 8); `route` → `Focus` (`unminimize` + `set_focus`), `Deliver` (`emit_to(origin, "document-ready", ())` + `set_title`), `NewWindow` (`create_reader`, como hoy).
- `error.rs`: `pub fn batch_message(errs: &[AppError]) -> Option<String>` (pura, H10) y `report_batch(app, origin, errs, detail)`, que registra cada error y muestra un solo diálogo; `report` delega en él.
- `lib.rs`: `ventana_inicial` crea `reader-0` («Miso») y, si hay rutas, `block_on(open_paths("reader-0", rutas))`. `rutas(args: impl IntoIterator<Item = impl Into<OsString>>, cwd: &Path) -> Vec<PathBuf>` pura (sin `-…`, relativas contra `cwd`); sustituye a `primer_argumento`. `on_window_event`: `DragDrop(DragDropEvent::Drop { paths, .. })` → `spawn(open_paths(label, paths))`.
- Frontend: `onDocumentReady(cb)` en `events.ts` (sustituye a `onDocumentOpened`); `Reader` → `takePendingDocument()` en el evento y al montar.
- Tests Rust: `Focus` con el archivo abierto y con el pendiente; `path` igual con `file_id` distinto → `Focus`; archivo movido (`file_id` igual, `path` distinta) → `Focus`; `take` conserva la clave (`key` tras `take`); origen `None` → `NewWindow`; `Deliver` deja pendiente recogible una vez; `batch_message` con 1 y 3 errores; `rutas` (relativa, absoluta, flags, vacío). Los tests de `Slots` existentes se adaptan.

**Gate:** `pnpm verify` verde.

### Paso 5 — Posiciones
**Archivos:** `src-tauri/src/positions.rs` (nuevo), `src-tauri/src/doc_windows.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
- `Positions { schema_version, entries: Vec<Entry> }` (H5), `Default` con versión 1. `fn find(&self, key) -> Option<usize>` (H4); `fn get(&self, key, flat_len) -> u32` (ya recortado, pura); `fn set(&mut self, key, char_index, now_ms)`: actualiza la entrada encontrada **reescribiendo su clave** (así una copia movida adopta el nuevo `file_id`/`path`) o añade; `fn prune(&mut self)` (tope 1000).
- `PositionsState { inner: Mutex<Inner { positions, dirty, scheduled }>, path }` (H6); ruta `app_data_dir()/positions.json`; `app.manage` en `setup` **antes** de `ventana_inicial`, cargado con `store::load` (`ok` = versión 1). `fn flush(&self)`: con el cerrojo, si `dirty`: `prune` + `store::save`; éxito → `dirty = false`; error → `log::error!`.
- `commands::set_position(app, window, char_index: u32)`, registrado en `generate_handler!`: sin clave (portapapeles) → nada; si no, `set` + `dirty` y, si no hay uno programado, hilo que duerme 5 s, apaga `scheduled` y hace `flush`.
- `open_doc`: con clave, `doc.initial_char_index = get(key, doc.flat_len())` **antes** de `route` (el `Doc` aparcado ya debe llevarlo) y `log::info!` con *label*, título e índice.
- `lib.rs`: `flush` en `WindowEvent::Destroyed` de una lectora y en `RunEvent::Exit` (`Builder::build(..)?.run(|app, event| …)`).
- Tests: `find` por `file_id`; por hash con desempate de `path` y, sin coincidencia de `path`, la más reciente; por `path` sola; `set` reescribe la clave; `get` recorta a `flat_len`; poda a 1000 conservando las más recientes; `schema_version: 2` → default + `*.corrupt`; `flush` sin cambios no escribe; `flush` con la ruta en un directorio imposible deja `dirty`.

**Gate:** `pnpm lint:rust`, `pnpm test:rust` verdes.

### Paso 6 — Portapapeles
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/src/clipboard.rs` (nuevo), `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`
- `arboard` (H11). `fn read_doc() -> Option<Doc>`: `Clipboard::new()?.get_text()` según H7; `Doc { title: "Portapapeles", initial_char_index: 0, blocks: plain::blocks(&text) }`.
- `async open_from_clipboard(app, window)`, registrado en `generate_handler!`: `spawn_blocking(read_doc)`; `Some` → `open_doc(origin, doc, None)`.
- `ventana_inicial`: sin rutas, tras crear `reader-0`, `read_doc()` → `open_doc("reader-0", …)` dentro de `setup` (el bucle de eventos aún no sirve IPC: la ventana recoge el pendiente al montar).
- Test: `read_doc` no es testeable sin portapapeles real; la conversión texto → `Doc` sí (texto con sólo blancos → `None`, extraído como función pura).

**Gate:** `pnpm lint:rust`, `pnpm test:rust` verdes.

### Paso 7 — Una sola instancia
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/doc_windows.rs`
- Plugin registrado **el primero**. Callback `(app, argv: Vec<String>, cwd: String)`: `rutas(argv.into_iter().skip(1), Path::new(&cwd))` (riesgo 9); vacías → enfocar la última lectora; si no, `spawn(open_paths(última, rutas))` (H9).
- `DocWindows.focused: Mutex<String>` actualizado en `WindowEvent::Focused(true)`; `fn last_reader(app) -> Option<String>` con respaldo a cualquier `reader-*`.

**Gate:** `pnpm lint:rust` y `pnpm test:rust` verdes.

### Paso 8 — Frontend: Ctrl+V y posición
**Archivos:** `src/lib/ipc/commands.ts`, `src/lib/nav/keyboard.ts`, `src/lib/nav/keyboard.test.ts`, `src/lib/nav/columnScroll.ts`, `src/components/Columns.svelte`, `src/components/Reader.svelte`, `tests/layout/anchor.svelte.test.ts`
- `commands.ts`: `openFromClipboard()`, `setPosition(charIndex)`.
- `keyboard.ts`: Ctrl+V → `paste`. `Reader`: la guarda del panel pasa a `if (inPanel && (action === 'paste' || !e.ctrlKey)) return`, antes de `preventDefault` (H7); fuera del panel, `openFromClipboard()`.
- `createColumnScroller(viewport, columns, starts, initialAnchor, onAnchor)`: `anchor` arranca en `initialAnchor`; `sync` llama a `onAnchor(anchor)` cuando cambia. `Columns` pasa `doc.initial_char_index` y una prop **opcional** `onanchor?: (i: number) => void` (los montajes de `columns.test.ts` no cambian); `Reader` → `setPosition`.
- Tests: `Ctrl+V` → `paste`, `Ctrl+Alt+V` → nada; layout (helpers `charLeft` y espera de `scrollend` de `anchor.svelte.test.ts`): montar con `initial_char_index` en mitad del documento y, tras esperar, el carácter queda en `[vp.left − 0.5, vp.left + pitch)` sin interacción y `onanchor` no se ha llamado; un `goTo` posterior lo llama una vez con el nuevo ancla.

**Gate:** `pnpm verify` verde.

### Paso 9 — Comprobación en la app
**Archivos:** ninguno (orquestador, `pnpm tauri dev`; sin verificación visual).
- Abrir un `.txt` largo por argumento, avanzar, cerrar: `positions.json` en `%APPDATA%\com.provisional.miso\` con la entrada (UTF-8, sin nulos). Reabrir: la línea `log::info!` de `open_doc` (P5) muestra el mismo índice. Con tres argumentos (uno bueno, dos inexistentes): una ventana con el bueno y un solo diálogo terminado en «Y 1 más.».
- Segunda instancia (`target\debug\miso.exe <ruta>` con la primera viva): abre en la primera; con la misma ruta, enfoca la ventana existente. Log sin errores.
- **Titular:** reabrir vuelve al punto exacto; Ctrl+V con texto copiado (ventana vacía → carga en ella; llena → ventana nueva); Ctrl+V en un campo del panel pega; arrancar con texto copiado y sin archivo lo muestra; soltar uno y varios archivos (y una carpeta: nada); soltar un archivo ya abierto enfoca su ventana; foco de la segunda instancia (riesgo 5).

### Paso 10 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, este plan
- Prueba negativa (revertida): `set` sin reescribir la clave hace fallar su test.
- `ARCHITECTURE.md`: §4 (`store.rs`, `clipboard.rs`, `file_id.rs` con `DocKey`), §5 (modelo sin `doc_key`, `initial_char_index: u32`, `set_position(char_index)`, `open_from_clipboard`, `document-ready` y H1), §8 (H3–H6), §9 (H8, H9), §13 (fase 3 hecha).
- `DIFERIDOS.md`: cerrar "Tests de `file_id`" y la confirmación de fase 2; añadir lo aparcado en §9, con fecha y hora.
- `REGISTRO.md`: asiento de fase 3. Este plan → `.historial/_PLAN_FASE_3_completado.md`.
- Commit `feat: fase 3 — posición persistente, portapapeles, drop y una sola instancia`.

**Gate:** `pnpm verify` verde sobre el commit final; `git status` limpio.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5 ─→ P6 ─→ P7 ─→ P8 ─→ P9 ─→ P10
```
P4 necesita `DocKey` (P3) y `read`/`from_bytes` (P2); P5 la clave por ventana (P4); P6 y P7 `open_doc`/`open_paths` (P4); P8 `set_position` (P5) y `open_from_clipboard` (P6).

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 sobre `d47b48f` (59 unit, 15 layout) | Base real `d47b48f` (Ctrl+rueda, posterior a `18e755f`) |
| P2 — Persistencia común y modelo | ✅ completado | `pnpm verify` = 0 (Rust 35) | Tests de `load`/`save` movidos a `store.rs` con tipo propio |
| P3 — Identidad de archivo | ✅ completado | lint + test Rust = 0 (41); `windows` 0.62.2 única | `canonicalize` deja prefijo `\\?\` en `path` (coherente en todas las vías) |
| P4 — Enrutado | ✅ completado | `pnpm verify` = 0 (Rust 54) | `import()` sólo en tests; ventana destino desaparecida → `Internal` |
| P5 — Posiciones | ✅ completado | `pnpm verify` = 0 (Rust 70) | `set_position` async |
| P6 — Portapapeles | ✅ completado | lint + test Rust = 0 (73) | `arboard` 3.7.0 reutiliza `windows-sys` 0.61.2 |
| P7 — Una sola instancia | ✅ completado | lint + test Rust = 0 (77) | Plugin 2.5.5; `last_reader` con `try_state` (sin pánico en FFI); `pick_reader` puro y testeado |
| P8 — Frontend | ✅ completado | `pnpm verify` = 0 (Rust 77, unit 63, layout 17) | Sin desviaciones |
| P9 — Comprobación en la app | ✅ parte del orquestador | Log y `positions.json`: tanda, guardado a 5 s, reapertura en 17337, segunda instancia, portapapeles al arrancar | Ctrl+V sin verificar (las pulsaciones sintéticas con Ctrl no llegan a WebView2); comprobaciones del titular en `DIFERIDOS.md` |
| P10 — Cierre y commit | ✅ completado | Negativa: sin reescribir la clave falla su test | |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- Pegar un archivo copiado en el Explorador (`CF_HDROP`; Tofu sólo acepta `file-url`): el texto entra por Ctrl+V.
- Asociaciones de archivo en `tauri.conf.json` (fase 7); la segunda instancia se prueba lanzando el ejecutable de `tauri dev`.
- `Doc.source`, `Doc.id`, `is_plain`, `lang` (entran con su consumidor).
- Soltar texto (§9.2 *verificar*: sigue sin soporte).
- Migraciones de `positions.json`.

**Hallazgos aparcados**
- (vacío)

## 10. Auditoría

2026-11-12, agy y OpenCode (Big Pickle), sólo lectura y sin respaldo físico; deliberación del orquestador (Opus).

- **Aplicado:** literales de `Doc` y `mod` nuevos en los archivos de P2/P3 (puertas rojas entre pasos); `import` se conserva como envoltorio; formato del `file_id` con `u128::from_le_bytes`; ventana de origen cerrada durante la importación (riesgo 8); `take` conserva la clave; `initial_char_index` antes de `route`; `get` puro con recorte; `PositionsState` registrado antes de `ventana_inicial`; un solo `Mutex` para el debounce (sin atómicos; `dirty` sobrevive a un fallo de escritura); `date_changed` en ms; `Entry` con `DocKey` aplanado; `path` como `String`; `canonicalize` con respaldo; `report_batch`; `rutas` genérica y sin indexar `argv`; comandos en `generate_handler!`; `onanchor` opcional; guarda de Ctrl+V en el panel; test de posición inicial con espera y tolerancia; tests de archivo movido y de `take`; `log::info!` del índice para P9; `arboard` sin `image-data`; gate de P7 con `test:rust`; `prune(&mut self)`.
- **Rechazado:** reescribir la clave sólo con coincidencia por `file_id` (H4 quiere que una copia movida de disco adopte su nueva clave; dos copias idénticas sin `file_id` comparten posición por diseño); *labels* consecutivos sin saltos (cosmético); respaldo determinista de `last_reader` (cualquier lectora viva sirve); helper puro `shouldHandle` para el panel (una condición de una línea; lo cubre P9).
