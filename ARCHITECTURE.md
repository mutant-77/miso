# Arquitectura — Miso, clon de Tofu (Tauri 2 + Svelte)

Base: `rea-tofu-evidence.md` (ingeniería inversa de Tofu 3.1). Este documento fija **qué se construye y dónde vive cada cosa**. Lo marcado con *(verificar)* es una decisión que depende de una medición o prueba pendiente. Las decisiones del titular viven en `DECISIONES.md`.

## 1. Producto

Lector de documento único por ventana. Toma texto de un archivo o del portapapeles y lo muestra en **columnas estrechas** que se leen de arriba abajo y de izquierda a derecha. Se navega **por columnas**, no por píxeles. Recuerda dónde lo dejaste.

Fuera de alcance (igual que Tofu): edición, biblioteca, sincronización, red, telemetría, auto-update, licencias, plugins. Además, a diferencia de Tofu: guardado RTFD e integración con el pasteboard de búsqueda.

## 2. Decisiones de fondo

| # | Decisión | Motivo |
|---|---|---|
| D1 | **El motor de columnas es CSS multi-column en el WebView**, no un layout propio. | Tofu necesita un `NSLayoutManager` + N `NSTextContainer` + pool de `NSTextView` porque AppKit no tiene multicol. En web, `column-width` + `column-fill: auto` + altura fija desborda en horizontal y el navegador reparte el texto y los saltos de línea. Se elimina el 80 % de la complejidad de `TofuDocumentView`. |
| D2 | **Rust parsea, Svelte pinta.** Rust convierte cualquier formato a un modelo neutro (`Doc`); el frontend lo renderiza. | Los importadores de `NSAttributedString` (RTF, HTML, DOCX, PDF) no existen en web; hay que sustituirlos y es trabajo de backend. Mantiene el WebView sin parsers pesados y sin ejecutar HTML ajeno. |
| D3 | **La posición de lectura es un índice de carácter sobre el texto aplanado**, no un scroll en píxeles. | Es la ancla de Tofu (`firstVisibleCharacterIndex`) y sobrevive a cambios de fuente, ancho y ventana. |
| D4 | **Una ventana = un documento**, como `NSDocument`. Estado por ventana en el frontend; estado global (prefs, posiciones, documentos abiertos) en Rust. | Fiel al original; Tauri 2 soporta N `WebviewWindow` con el mismo bundle. |
| D5 | **Svelte 5 (runes) + Vite + TypeScript, sin SvelteKit.** Versiones mayores fijadas: Tauri 2 (no la 3 alpha), TypeScript 6 (`svelte-check` 4.x no admite TS 7). | Es una SPA local sin rutas ni SSR. |
| D6 | **Sin red.** Capabilities mínimas, sin `http`/`shell`, CSP estricta. | Tofu no tiene red; es un rasgo del producto y reduce superficie. |
| D7 | **Windows primero (WebView2).** macOS/Linux no se descartan, pero no guían decisiones. | Plataforma de desarrollo y de uso. |
| D8 | **Unidades: todo en px CSS.** Los "pt" de Tofu se traducen 1:1 a px (360 pt → `360px`). Nunca se usa la unidad CSS `pt`. | En macOS 1 pt ≈ 1 px lógico; `360pt` en CSS serían 480 px. |
| D9 | **Índice de carácter en unidades UTF-16**, sobre texto con saltos normalizados a `\n`. | Es la unidad del DOM (`Range`, `Text.length`). Rust debe contar con `encode_utf16()`, no con `chars()`. |
| D10 | **Toda E/S de archivos y portapapeles ocurre en Rust.** El frontend nunca envía rutas: diálogos, drops y portapapeles se gestionan en Rust. | Los comandos propios de Tauri no pasan por el *scope* de capabilities; así no hay comando que lea o escriba una ruta arbitraria. |
| D11 | **Los errores los registra y presenta Rust** con un diálogo nativo de mensaje (`report()` en `error.rs`). El frontend nunca muestra errores. | Funciona antes de que exista una ventana, no toca la maqueta de columnas, no requiere permiso `dialog` en el frontend ni textos de error en Svelte. Tofu hace lo mismo con su aviso de sistema. |

## 3. Mapa Tofu → clon

| Tofu (ObjC) | Clon | Capa |
|---|---|---|
| `AppController` | `src-tauri/src/lib.rs` (setup, plugins, menú, arranque con portapapeles) | Rust |
| `TofuDocument` (lectura) | `src-tauri/src/import/*` → `Doc` | Rust |
| `TofuDocument` (guardado) | `src-tauri/src/export.rs` | Rust |
| `TofuDocumentController` | `src-tauri/src/doc_windows.rs` (enrutar documentos, crear ventana, cerrar la vacía inicial) | Rust |
| `TofuWindowController` / `TofuWindow` | `doc_windows.rs` + `src/components/Reader.svelte` (fullscreen) | ambas |
| `TofuDocumentView` | `Columns.svelte` + `lib/layout/*` (CSS multicol + ancla) | Svelte |
| `TofuScrollView` | `lib/nav/columnScroll.ts` | Svelte |
| `TofuTextView` (teclas, rueda, paste) | `lib/nav/keyboard.ts`, `lib/nav/wheel.ts` | Svelte |
| `TofuProgressIndicator` | `LoadingOverlay.svelte` | Svelte |
| `PreferenceController` | `src-tauri/src/prefs.rs` + `Preferences.svelte` | ambas |
| `FindPanelController` + `IncrementalTextFinder` | `lib/find/*` + `FindBar.svelte` | Svelte |
| `DocumentPropertiesManager` | `src-tauri/src/positions.rs` | Rust |

## 4. Estructura del repositorio

```
Miso/
├─ ARCHITECTURE.md
├─ assets/miso.svg              # fuente del icono (`pnpm tauri icon`)
├─ .gitignore / .gitattributes  # LF; fuera de git: Tofu.dmg, rea-tofu-evidence.*, builds
├─ .cargo/config.toml           # TS_RS_EXPORT_DIR (en la raíz: Cargo busca la config desde el directorio actual)
├─ package.json                 # pnpm; scripts dev / build / check / test / test:layout / lint:rust / test:rust / gen:licenses / check:licenses / verify
├─ scripts/licenses.mjs         # manifiesto de licencias; genera licenses_data.rs y THIRD_PARTY_NOTICES.md (--check en verify)
├─ THIRD_PARTY_NOTICES.md       # generado; no editar a mano
├─ .npmrc                       # node-linker=hoisted (el volumen es exFAT, sin symlinks)
├─ vite.config.ts
├─ vitest.config.ts             # proyectos unit (node) y layout (navegador, ver sección 12)
├─ svelte.config.js
├─ tsconfig.json
├─ index.html
├─ locales/                     # catálogos de interfaz (en, es, pt, de, it, fr, nl, ru, nb, sv, fi), compartidos por Rust y Svelte
├─ src/                         # frontend
│  ├─ main.ts
│  ├─ App.svelte                # monta Reader (no hay ventana de preferencias)
│  ├─ lib/
│  │  ├─ i18n.svelte.ts         # idioma de la interfaz ($state), t(), endónimos
│  │  ├─ ipc/
│  │  │  ├─ commands.ts         # wrappers tipados de invoke()
│  │  │  ├─ events.ts           # listen() tipado
│  │  │  └─ types/              # generados por ts-rs en `cargo test` (un archivo por tipo); no se editan
│  │  ├─ layout/
│  │  │  ├─ columnWidth.ts      # regla de ancho de columna (+ umbral √2 en fullscreen)
│  │  │  ├─ anchor.ts           # charIndex <-> posición visible (caretPositionFromPoint)
│  │  │  ├─ composition.ts      # clases de composición de libro por bloque (sep, indent, chapter, blank)
│  │  │  └─ textIndex.ts        # índice de carácter global sobre los bloques
│  │  ├─ nav/
│  │  │  ├─ columnScroll.ts     # scroll animado por columna, respeta prefers-reduced-motion
│  │  │  ├─ keyboard.ts         # mapa de teclas (sección 7)
│  │  │  └─ wheel.ts            # rueda/trackpad -> scroll horizontal + ancla
│  │  └─ find/
│  │     ├─ search.ts           # coincidencias sobre el texto aplanado (flatText) y siguiente desde un índice
│  │     └─ highlight.ts        # Match -> Range y resaltado 'find' (CSS Custom Highlight API)
│  └─ components/
│     ├─ Reader.svelte          # raíz de la ventana de lectura; carga y escucha prefs, escala de fuente (sin state/ ni theme/)
│     ├─ Columns.svelte         # contenedor multicol + pintado de bloques (párrafo, heading, lista, cita, imagen)
│     ├─ FindBar.svelte         # barra flotante arriba a la derecha (Ctrl+F), «n de m»
│     ├─ Preferences.svelte     # panel lateral superpuesto dentro del lector (Ctrl+,), colores del sistema; vista «Licencias»
│     ├─ Licenses.svelte        # avisos de licencia embebidos (get_licenses), seleccionables
│     ├─ EmptyState.svelte      # "abre un archivo o pega texto"
│     └─ LoadingOverlay.svelte
├─ src-tauri/
│  ├─ Cargo.toml                # única fuente de la versión (tauri.conf.json no la declara)
│  ├─ tauri.conf.json           # bundle id, CSP; sin instalador ni asociaciones; sin ventanas (las crea Rust)
│  ├─ capabilities/
│  │  └─ reader.json            # sólo core (eventos, ventana); sin fs, dialog, clipboard, red ni opener
│  ├─ pdfium/                   # pdfium.dll x64 (chromium/8125) + LICENSE + licenses/, versionados; embebida en el .exe
│  └─ src/
│     ├─ main.rs
│     ├─ lib.rs                 # builder, plugins (dialog, log, single-instance), registro de comandos, protocolo `miso-img`, panic hook
│     ├─ commands.rs            # #[tauri::command] finos; delegan en módulos
│     ├─ doc_windows.rs         # enrutado de documentos, ventanas, Doc pendiente/abierto por label, imágenes por número (`image_response`)
│     ├─ model.rs               # Doc, Block, Run (serde + specta/ts-rs para generar types.ts)
│     ├─ licenses_data.rs       # generado por scripts/licenses.mjs: include_str! de cada licencia
│     ├─ import/
│     │  ├─ mod.rs              # despacho (%PDF- → PDF, {\rtf → RTF, zip con container.xml → EPUB, otro zip → DOCX, extensión → HTML, resto → texto), helpers de zip/XML compartidos (entry, attr, entity, corrupt, LIMITE, Parsed), clean, push_run, catch_unwind → Corrupt
│     │  ├─ plain.rs
│     │  ├─ rtf.rs              # lector propio: tokenizador + pila de estado por grupo
│     │  ├─ html.rs             # scraper -> Doc
│     │  ├─ docx.rs             # zip + quick-xml sobre document.xml, styles.xml y numbering.xml
│     │  ├─ epub.rs             # zip + quick-xml: container.xml → OPF → spine; capítulos por html.rs; imágenes → Doc.images
│     │  └─ pdf.rs              # pdfium-render; párrafos reconstruidos por geometría
│     ├─ file_id.rs             # DocKey: identidad de archivo NTFS/ReFS + hash FNV-1a de respaldo + ruta canónica
│     ├─ i18n.rs                # catálogos incrustados, detección del idioma del sistema, idioma vigente, t()
│     ├─ fonts.rs               # familias instaladas (DirectWrite) para el desplegable de Preferencias
│     ├─ clipboard.rs           # texto del portapapeles (arboard) -> Doc «Portapapeles»
│     ├─ store.rs               # load/save JSON atómicos con *.corrupt (prefs y posiciones)
│     ├─ export.rs              # copia en texto (UTF-8, CRLF) y RTF
│     ├─ prefs.rs               # defaults, validación por recorte (§8), persistencia atómica, apply + prefs-changed
│     ├─ positions.rs           # posiciones de lectura, debounce 5 s
│     └─ error.rs               # AppError + report() (log + diálogo nativo); único lugar con texto de error visible
└─ tests/
   ├─ fixtures/                 # un archivo por formato + casos rotos
   ├─ layout/                   # tests de layout en Chromium real (sección 12)
   └─ e2e/                      # sólo si se decide (sección 12)
```

**Entorno (Windows):** compilar Rust y ejecutar las puertas (`pnpm verify`, `pnpm tauri dev`) **desde PowerShell**. En Git Bash, `link.exe` resuelve al de Git (`usr/bin`) y no al enlazador MSVC. El repositorio está en un volumen exFAT: los avisos de Cargo "hard linking files in the incremental compilation cache failed" son inofensivos. Puerta única: `pnpm verify`.

**Versión (SemVer):** mayor si se rompe algo que el usuario tenía (prefs o posiciones ilegibles sin migración, formato o Windows retirados); menor si hay función nueva compatible; parche si sólo hay correcciones. Procedimiento: cambiar `version` en `src-tauri/Cargo.toml` → `pnpm verify` → asiento en `REGISTRO.md` y commit «Versión X.Y.Z» → `git tag -a vX.Y.Z` → el titular compila → push de la etiqueta y Release en GitHub con `miso.exe` y notas sacadas de `REGISTRO.md`. Acerca de la muestra con `getVersion()` (permiso `core:app:allow-version`).

## 5. Contrato IPC

Los tipos se **generan desde Rust con `ts-rs` 12** (`specta` seguía en *release candidate*) al ejecutar `cargo test`, en `src/lib/ipc/types/`. `pnpm verify` corre los tests de Rust antes de `check`. El modelo crece por fases: cada campo entra con la fase que lo usa (fase 1: `Doc { title, blocks }`, `Paragraph { runs }`, `Run { text }`; texto plano = un `Paragraph` por línea, salvo si viene cortado a ancho fijo (≥ 50 caracteres, la mayoría de los saltos con la palabra siguiente sin caber): esos saltos se unen con un espacio, lo que conserva los índices de carácter; fase 3: `initial_char_index: u32`).

### Modelo

```
Doc      { title, initial_char_index: u32, composition: "Text" | "Book", chapters: u32[], blocks: Block[] }
Block    = Paragraph{ runs } | Heading{ level, runs } | ListItem{ depth, runs } | Quote{ runs } | Image{ id }
Run      { text, bold?, italic?, underline? }   // estilos ausentes si son falsos: el texto plano serializa sólo `text`
```

- La clave de documento (`DocKey`, sección 8) sólo existe para archivos y vive en Rust, por ventana; el frontend no la conoce. `initial_char_index` (0 = inicio) lo resuelve Rust al abrir, a partir de la posición guardada.
- **Composición y capítulos:** `composition` es `Text` en texto plano y portapapeles, `Book` en el resto (lo decide `import::parse`). `chapters` = índices de bloque donde empieza un capítulo (crecientes, nunca 0); sólo EPUB los rellena. Un capítulo no es un bloque: no suma texto ni mueve el índice.
- **Estilos básicos (fase 5):** estructura + negrita/cursiva/subrayado. Sin tamaños, colores, fuentes ni idioma del documento (diferido).
- **Sin `\n` dentro de un bloque:** `<br>`, `w:br` y `w:cr` parten el bloque; un `ListItem` partido sigue sin marcador. Los marcadores de lista son texto del run (`"• "`, `"3. "`): se indexan, se buscan y se copian.
- **Contenido no representado:** cuadros de texto, notas y comentarios se omiten; las imágenes sólo se leen en EPUB (DOCX y HTML suelto las omiten); las tablas se aplanan a un párrafo por fila (celdas separadas por tabulador, párrafos de una celda unidos por un espacio); los enlaces se pintan como texto, sin abrirse.
- **Imágenes (EPUB):** `Block::Image { id }` cuenta 0 unidades UTF-16 (su separador `\n` cuenta como el de cualquier bloque); en el DOM es `<p data-char-start class="img"><img></p>`. `Doc.images` (`Image { id, mime, bytes }`) sólo existe en Rust y no viaja por IPC. Los `id` son únicos en el proceso y una misma ruta dentro de un libro da el mismo `id`. El frontend compone la URL con `imageSrc(id)` (protocolo `miso-img`): `image_response` sirve sólo las imágenes de la ventana que pide, por número; otra ventana, ventana cerrada o id desconocido → 404.

El texto plano aplanado (concatenación de runs con separador de bloque) define el **índice de carácter** global (D3). Reglas fijas para que Rust y TS den el mismo número:

- Unidad: **UTF-16 code units** (D9). En Rust, `s.encode_utf16().count()`.
- Antes de indexar, Rust normaliza `\r\n` y `\r` a `\n` y elimina `U+FFFC`. El frontend nunca recibe texto sin normalizar.
- Separador de bloque: exactamente un `\n` (cuenta 1). Los importadores ricos pasan su texto por `import::clean` (quita `\r`, `U+FFFC`, `U+00AD`; `\n` → espacio).
- El DOM no puede añadir texto: nada de espacios entre `Block`s, ni `::before`/`::after` con contenido textual. Única excepción: los guiones blandos (U+00AD) de la partición, que `anchor.ts` descuenta al traducir entre DOM e índice; por eso los importadores eliminan los U+00AD del original.
- Test cruzado sobre un fixture con emoji (pares sustitutos), CJK, combinantes, CRLF y `U+FFFC`.

### Enrutado de documentos

Toda apertura (diálogo, "abrir con", drop, portapapeles) pasa por `doc_windows.rs`:

1. Si el archivo ya está abierto o pendiente (misma `path` **o** mismo `file_id`; el hash no cuenta), se enfoca su ventana y no se abre otra.
2. Si la ventana de origen está vacía, el `Doc` queda **pendiente** para ella y se le emite `document-ready` (sin carga); responde con `take_pending_document`.
3. Si no (o si la ventana de origen se cerró durante la importación), Rust crea una ventana nueva con el `Doc` pendiente para su `label`, que lo recoge al montar.

Toda entrega pasa por el pendiente: `take_pending_document` lo devuelve una sola vez, así que recogerlo por el evento y al montar es inocuo y no hay carrera con `listen()`. Arranque, Ctrl+O, drop y segunda instancia comparten `open_paths` (en orden, como una tanda).

Rust conserva el `Doc` de cada ventana abierta (lo necesitan `save_copy` y la regla 1) y lo libera al cerrarla.

Los importadores corren fuera del hilo principal (`spawn_blocking`), sin excepción.

### Comandos

| Comando | Entrada | Salida |
|---|---|---|
| `take_pending_document` | — | `Doc \| null` |
| `open_dialog` | — | — (Rust muestra el diálogo y enruta; desde la fase 1) |
| `open_from_clipboard` | — | — (Rust lee el portapapeles y enruta; sin contenido válido no hace nada) |
| `get_prefs` | — | `Prefs` |
| `set_prefs` | `Prefs` completo | `Prefs` validado |
| `reset_prefs` | — | `Prefs` |
| `list_fonts` | — | `string[]` (familias instaladas, ordenadas, ya válidas para prefs; sin `@` verticales; vacía si DirectWrite falla) |
| `get_licenses` | — | `License[]` (`title`, `kind` SPDX, `body` con saltos `\n`), embebidos en el `.exe` |
| `set_position` | `char_index` | — (Rust resuelve la clave por la ventana que llama; sin clave, nada; debounce en Rust) |
| `save_copy` | — | — (Rust muestra el diálogo de guardado y exporta el `Doc` de la ventana; formato por la extensión elegida: `.rtf` → RTF, si no texto) |

### Eventos (Rust → ventanas)

`prefs-changed` (a todas las ventanas, equivale a `columnDimensionSettingsChanged:` / `backgroundColorChanged:`), `document-ready` (sin carga, sólo a la ventana de origen tras dejarle un `Doc` pendiente, ver enrutado). No hay evento de error (D11). Cada ventana escucha con `getCurrentWebviewWindow().listen`, **nunca** con el `listen` global, que recibe también los `emit_to` dirigidos a otras ventanas. Las ventanas de lectura (`reader-{n}`) las crea Rust. Con argumentos de lanzamiento se importan antes de abrir ninguna ventana, y cada una nace con su `Doc` pendiente; si ninguno abre, `reader-0` vacía. Sin argumentos, `reader-0` y después el portapapeles con ella como origen. El frontend no muestra el estado vacío hasta resolver el primer `take_pending_document`.

### Errores

Todo `AppError` pasa por `report(app, ventana_origen?, err)`: lo registra en el log y muestra un diálogo nativo de mensaje (`tauri-plugin-dialog`, invocado desde Rust), colgado de la ventana de origen si existe *(verificar que `parent` y el `show` no bloqueante funcionan así en Windows)*. Los comandos devuelven además el `Err` ya presentado, serializado como `{ kind, message }`; el frontend sólo lo usa para restaurar su estado de carga y nunca lo muestra.

| Variante | Cuándo | Presentación | Fase |
|---|---|---|---|
| `Read` | archivo inexistente, sin permiso o bloqueado | diálogo + log | 1 |
| `Unsupported` | formato sin importador (en fase 1: todo lo que no es texto, p. ej. bytes nulos) | diálogo + log | 1 |
| `Internal` | fallo al crear ventana o emitir, label desconocido | diálogo genérico + log (detalle sólo en log) | 1 |
| `Persist` | fallo al escribir `prefs.json` / `positions.json` | prefs: diálogo **una vez por sesión** + log; posiciones: sólo log | 2 / 3 |
| `Corrupt` | un importador rico falla (zip o XML inválido, EPUB sin OPF, rootfile o spine, parte de más de 256 MB, PDF que PDFium no carga) o entra en pánico (`catch_unwind` en `import::from_bytes`). El RTF nunca falla: conserva lo leído | diálogo + log (detalle sólo en log) | 5 |
| `Protected` | PDF con contraseña (`PasswordError` de PDFium); EPUB con un capítulo cifrado en `META-INF/encryption.xml` (DRM). Sólo fuentes ofuscadas no cuentan: el libro abre | diálogo + log | 6 |
| `Write` | fallo de `save_copy` | diálogo + log | 7 |

- **Texto visible:** un único `match` en `AppError::user_message()`. Muestra el nombre del archivo, nunca la ruta; el mensaje de E/S lo da el sistema operativo, sin el sufijo ` (os error N)` (`os_message`). Los textos salen del catálogo del idioma vigente (ver «Idioma de la interfaz», §8).
- **Fallo al guardar prefs:** el cambio se aplica igualmente en la sesión y `set_prefs` no devuelve `Err`.
- **Arranque rápido:** `create_reader` inyecta las prefs con `initialization_script` (`window.__PREFS__`) antes del bundle; el primer fotograma sale con ellas sin IPC. `getPrefs` (en paralelo con `take_pending_document`) sólo reconcilia un cambio ocurrido mientras se abría la ventana.
- **No son errores** (no se muestra nada): cancelar un diálogo de archivo, portapapeles vacío o sin texto (si está ocupado por otra app, sólo log), archivo de 0 bytes (documento vacío).
- **Varios fallos de una tanda** (drop múltiple, argumentos de lanzamiento): un solo diálogo con el primero y «Y N más.»; todos al log (`report_batch`, mensaje puro en `batch_message`).
- **Log:** `tauri-plugin-log` al directorio de logs de la app, con tamaño limitado; además a consola en desarrollo. `report()` registra la ruta completa y la causa. Un `panic::set_hook` registra los pánicos. Los errores JS del frontend sólo van a la consola del WebView.

## 6. Motor de columnas (frontend)

```
Reader                  fondo, texto, fuente (prefs); {#key doc}
└─ .frame               100 %; custom properties desde prefs y escala; --vp-w
   └─ .viewport         width: var(--vp-w); margin-inline: auto; overflow-x: auto; overflow-y: hidden;
                        overflow-anchor: none; scrollbar-width: thin (color del texto)
      └─ .columns       height: 100%; box-sizing: border-box; lang = idioma del sistema;
                        column-width: var(--col-w);
                        column-gap: var(--gap);         /* 2 × inset + 10 */
                        column-fill: auto;
                        padding: var(--v-inset) var(--pad);
         └─ Block*      h1..h6: text-align: center; break-after: avoid
                        .book p.sep / .indent / .chapter / .blank (composición de libro)
```

- **Viewport exacto.** `--vp-w = fitViewport(ancho de .frame, columnWidth + 2 × inset)` (`columnWidth.ts`), escrito de forma imperativa en `.frame` al montar, en su `ResizeObserver` y tras cada cambio de prefs/escala: un `$state` intermedio dejaba correr el `$effect` antes de que el DOM tuviera el ancho nuevo.

- **Paso de columna real (crítico).** En CSS `column-width` es un **mínimo**: el navegador estira las columnas para llenar el ancho del contenedor (`usado = (W + gap) / N − gap`, con `N = floor((W + gap) / (w + gap))`). Por eso **nunca** se usa el ancho de la preferencia para calcular el scroll. Dos mecanismos combinados:
  1. **Fuente de verdad = DOM.** Tras cada layout se mide el paso real (`pitch = colUsada + gap`) con `getClientRects()` de un `Range` en dos columnas consecutivas (o de la primera línea de dos columnas). `columnScroll`, `anchor` y `wheel` usan sólo ese `pitch`.
  2. **Ancho fijo en ventana normal.** El viewport se ajusta a un múltiplo exacto del paso: `W = N × (columnWidth + 2 × inset)` (centrado, el sobrante queda como margen). Así la columna mide exactamente la preferencia, como Tofu con `setupResizingIntervals`. En pantalla completa manda la regla √2 y se fija `column-count` / `column-width` al valor calculado.
  - Test: tras 500 saltos de columna, el borde izquierdo del viewport coincide con el inicio de una columna (error < 1 px).
- **Ancho de columna.** Ventana normal: `columnWidth = pref.columnWidth` (360 px por defecto, D8). Pantalla completa: se aplica la regla observada de Tofu — `n = ancho / pref.columnWidth`; si `n / floor(n) <= 1.4142` se usan `floor(n)` columnas, si no `floor(n)+1` (mínimo 1); el ancho de columna es el que hace que `columnas` pasos llenen el ancho exacto (`fullscreenColumnWidth` en `columnWidth.ts`, con tests de la tabla de casos). `fit()` es el único sitio que escribe `--vp-w`, `--col-w`, `--pad` y `--gap`, con ese ancho o con el de la pref; el viewport exacto, el `pitch` y el ancla no cambian.
- **Sangrías y paso.** Las sangrías (listas, citas) son `padding`, nunca `margin`, y `measurePitch` mide las cajas de los `<p>` (un rectángulo por fragmento de columna), no las líneas: así el borde de cada caja es el de su columna. Títulos con su propio interlineado (`--lh × 1.2` sobre su tamaño): el heredado de `.columns` es una longitud fija y con interlineado bajo quedaría por debajo de su letra.
- **Inset horizontal relativo:** `inset = relativeInset × 0.01 × columnWidth` (3 % por defecto).
- **Equivalencia con AppKit** (ajustada a ojo contra Tofu, 2026-11-12). AppKit añade 5 px de `lineFragmentPadding` a cada lado del texto de cada columna: `column-width = columnWidth − 10`, `padding-inline = inset + 5`, `column-gap = 2 × inset + 10`. El paso sigue siendo `columnWidth + 2 × inset` (381.6 px por defecto). Interlineado: `line-height: calc(LineHeight × 1lh)` con `line-height: normal` en el padre, porque `lineHeightMultiple` multiplica el alto natural de la fuente, no su tamaño (Palatino Linotype 14 px: 19 → 20.9 px). En `Text` un bloque vacío (línea en blanco) mide media línea (`p.blank`, 0.5lh) (decisión del titular, 2026-11-13); en `Book`, 0 (ver «Composición de libro»).
- **Composición de libro** (`DECISIONES.md`, 2026-11-14). Sólo CSS sobre los `<p>` existentes: ni nodos, ni texto, ni `::before` (D3/D9). `bookClasses(doc)` (`composition.ts`) da las clases por bloque; `prev` = último bloque no blanco del mismo capítulo. `blank`: bloque sólo de espacios (incluye U+00A0), alto 0 sin `display: none` (el ancla necesita sus rectángulos). `sep`: `margin-block-start: 0.5lh`, salvo entre ítems de lista; es margen y no padding porque Chromium lo trunca en un salto de columna no forzado (sin hueco arriba). `indent`: `text-indent: 1.5em` en un `Paragraph` que no sigue a título ni imagen. `chapter`: primer bloque no blanco de cada capítulo, `break-before: column` y `margin-block-start: 0` (tras un salto forzado el margen se conserva). `.columns` lleva `book` si `composition === 'Book'`; en `Text` nada cambia salvo el centrado de títulos. Ancla, `measurePitch`, búsqueda y resaltado no cambian.
- **Ancla deseada** (`anchor.ts`, como `_desiredFirstVisibleCharacterIndex`). `firstVisibleCharIndex` usa `caretPositionFromPoint` en el inicio de la primera línea visible y `data-char-start` del `<p>`; `columnOfCharIndex` localiza la columna con un `Range` de un carácter. Tras un reflow (`ResizeObserver` del viewport, cambio de prefs o escala) `restore()` desplaza a la columna del ancla y anota la posición resultante (`anchoredLeft`). El ancla sólo se recalcula en un `scrollend` que acaba en otra posición: los desplazamientos de restauración no la tocan (sin deriva; lo prueba un test de 20 cambios alternos).
- **Scroll por columna.** `columnScroll.ts` guarda una **columna objetivo** `k` y desplaza a `round(k × pitch)` (tope `maxScroll`) con `scrollTo({ behavior })` nativo (`'instant'` con `prefers-reduced-motion`). Como el destino es siempre un múltiplo del `pitch`, no se acumula error. Tras un scroll ajeno (rueda), `scrollend` recalcula `k`.
- **Rueda.** En un contenedor con `overflow-x` el navegador no convierte la rueda vertical en scroll horizontal: `wheel.ts` traduce `deltaY` a `scrollLeft` (y respeta `deltaX` del trackpad).
- **Partición de palabras (decisión del titular, 2026-11-13):** `hyphens: auto` no funciona en WebView2 (sonda de fase 2: sin diccionarios), así que `hyphenate.ts` inserta guiones blandos al pintar con el paquete `hyphen` (patrones incluidos, sin red), palabras de 6+ letras. Idioma detectado por palabras frecuentes (`en`, `es`); si no es claro, no se parte. Los guiones blandos sólo existen en el DOM: el índice (D9), la búsqueda y el texto copiado no los ven. El guion visible de una palabra partida es otro rectángulo en la misma línea: `measurePitch` lo ignora y `charRect` usa el último rectángulo del carácter (el primero puede ser ese guion, en la línea o columna anterior).
- **Imágenes (EPUB).** `p.img` tiene `break-inside: avoid`; `img:not([hidden])` es `display: block` con `max-width: 100%` y `max-height: var(--col-h)` (mantiene la proporción). `fitHeight()` escribe `--col-h` = alto del viewport − 2 × inset vertical; lo llaman `fit()` y el `ResizeObserver` del viewport, que lo actualiza antes de `restore()` (la barra de scroll cambia esa altura; los anchos no se tocan ahí). Tras montar, espera a `img.decode()` de todas (la que falla queda `hidden`) y hace **un** `fit()` + `restore()`: la imagen mide 0 hasta cargarse, y un `restore()` por cada `load` maquetaría el libro N veces.
- **Rendimiento en documentos largos** *(verificar)*. Un único multicol obliga al navegador a maquetar todo el documento. Plan: medir con un texto de ~1 M de caracteres. Si el layout inicial supera lo aceptable, trocear en **segmentos** (grupos de bloques, cada uno su propio multicol, montados bajo demanda según el segmento visible). El contrato de `Columns.svelte` no cambia: sigue exponiendo `scrollToCharIndex()` y `firstVisibleCharIndex`.

## 7. Navegación

| Entrada | Efecto |
|---|---|
| ← / → | columna anterior / siguiente |
| ↑ / ↓ | igual que ← / → (una columna). *Desviación deliberada de Tofu, que mueve una línea dentro de la columna; el lector no tiene cursor.* |
| Re Pág / Av Pág | una “página” de columnas |
| Espacio | avanza el ancho visible |
| Inicio / Fin | principio / final del documento |
| Rueda / trackpad | scroll continuo; al soltar, actualiza el ancla |
| Redimensionar | reflow + restaurar ancla |

### 7.1 Atajos de aplicación

| Atajo | Efecto |
|---|---|
| Ctrl+O | `open_dialog` |
| Ctrl+V | `open_from_clipboard` (como `paste:` de Tofu) |
| Ctrl+F / F3 / Mayús+F3 | abrir búsqueda (o enfocarla) / siguiente / anterior; F3 con la barra cerrada la reabre con el último término. En el campo, Intro / Mayús+Intro = siguiente / anterior y Esc cierra y quita el resaltado |
| Ctrl+, | abre / cierra el panel de preferencias (Esc lo cierra) |
| Ctrl+I | alterna tema Claro / Oscuro según la luminancia del fondo actual; persiste como cualquier pref |
| Ctrl+S | `save_copy` |
| Ctrl+= / Ctrl+− / Ctrl+0, Ctrl+rueda | escala de fuente de la ventana: aumentar / reducir / restablecer (como `increaseFontSize:` / `decreaseFontSize:`). ×1.1 por paso, rango 0.5–3, sólo el tamaño de fuente. Por ventana, no persiste, no cambia prefs |
| F11 / Esc | alternar / salir de pantalla completa (`setFullscreen`, permiso `core:window:allow-set-fullscreen`). Esc cierra antes Preferencias y la barra de búsqueda. F11 funciona también en los paneles y sin documento. Sin aviso de pantalla completa |

## 8. Persistencia

- **Prefs:** `prefs.json` en el directorio de datos de la app (Rust), escritura atómica (temp + rename). Defaults de Tofu salvo la fuente: `Literata` (decisión del titular; variable con peso y tamaño óptico, OFL, latín, latín extendido y cirílico incluidos en `src/assets/fonts/` y declarados en `src/fonts.css`; el lector pinta a peso 450 para acercarse al trazo de macOS; `main.ts` la carga antes de montar para que el paso se mida con ella; el frontend compone `"<nombre>", serif`; el campo es un `<select>` con Literata, la familia actual y las de `list_fonts`; no un `<datalist>`, que WebView2 filtra por el texto escrito) 14 px, columna 360 px, inset relativo 3 %, inset vertical 20, interlineado 1.1, fondo blanco / texto negro. Alineación forzada, `ForcePreferredFont`, `ForceForegroundColor` y aviso de pantalla completa no existen (fase 5: ver «Fuente aplicada» y §7.1). Validación al cargar y al guardar, por **recorte** (cadena inválida → default del campo): fuente 8–48 px; columna 200–800 px; inset horizontal 0–20 %; inset vertical 0–80 px; interlineado 1.0–2.0; colores `#rrggbb`; familia = un solo nombre de 1–100 caracteres sin `" ; { } < > \`; `language` = `"auto"` o un código de `i18n::LANGS`.
- **Idioma de la interfaz (2026-11-15):** 11 idiomas, catálogos planos en `locales/<código>.json` (Rust con `include_str!`, Svelte con import). Sin plurales ni bibliotecas: las frases se redactan para no necesitarlos. `language` en prefs: `"auto"` (defecto) o un código. Con `"auto"`, el primer idioma soportado de la lista de preferencias del sistema (`sys-locale`; `no`/`nn` → `nb`), si no inglés. Rust fija el idioma vigente al cargar prefs (antes de la primera ventana) y en `apply`; `create_reader` inyecta `window.__SYSTEM_LANG__` y el frontend aplica la misma regla. `Restablecer` conserva el idioma. Fuera: botones de diálogos nativos y `os_message` (idioma de Windows), log (español), partición e idioma del documento.
- **Fuente aplicada.** Todo documento, plano o rico, se pinta con la fuente, el tamaño y los colores de prefs; los títulos escalan sobre ese tamaño. Por eso no existen `ForcePreferredFont`, `ForceForegroundColor`, `mostUsedFontSize` ni la alineación forzada (siempre justificado, títulos centrados); fuentes, tamaños y colores del documento están diferidos. La escala de ventana (7.1) multiplica el resultado.
- **Posiciones:** `positions.json` en el mismo directorio: `{ schema_version: 1, entries: [{ file_id?, content_hash, path, char_index, date_changed }] }` (`date_changed` en ms UTC). Otra versión se trata como corrupta; sin migraciones hasta que exista la 2. Tope de 1000 entradas: al guardar se podan las de `date_changed` más antiguo. Escritura atómica (temp + rename), **debounce de 5 s** (un hilo `std::thread` y un solo `Mutex` con `dirty`/`scheduled`; el guardado ocurre dentro del cerrojo y `dirty` sólo se apaga si salió bien) y flush al destruirse una ventana lectora y en `RunEvent::Exit`.
- **Clave de documento.** En macOS Tofu usa *security-scoped bookmarks* para localizar el archivo aunque se mueva. En Windows el equivalente más cercano es el **ID de archivo**: `GetFileInformationByHandleEx` con `FileIdInfo` → número de serie del volumen + ID de 128 bits (cubre NTFS y ReFS) (`file_id.rs`, vía `windows-rs`). Se mantiene al renombrar, al mover dentro del mismo volumen y al editar el archivo en sitio. Cada entrada guarda:
  - `file_id` (volumen + ID): **identidad principal**.
  - `content_hash` = FNV-1a de 64 bits sobre el tamaño (`u64` LE) y los primeros 64 KB, escrito a mano (el `DefaultHasher` de std no es estable entre versiones): **respaldo** cuando el ID no está o no coincide y el contenido no cambió (copia a otro disco, unidades FAT/exFAT o de red).
  - `path`: `fs::canonicalize` (forma `\\?\…`; si falla, la ruta recibida) como `String`: **último respaldo** y desempate. Cubre a los editores que guardan escribiendo un archivo nuevo y renombrándolo (cambian ID y contenido a la vez).
  - `date_changed`: para podar por antigüedad/cantidad.

  Búsqueda: primero por `file_id`; si no hay coincidencia, por `content_hash`; si no, por `path` exacta. Si varias entradas comparten hash, gana la que tenga la misma `path` y, si ninguna, la más reciente. Al guardar, la entrada encontrada adopta la clave actual (un archivo movido actualiza su `file_id`/`path`). Si el documento es ahora más corto que el `char_index` guardado, se limita al final.
- **Portapapeles:** no tiene posición persistente.
- **Fallos (D11):** si `prefs.json` o `positions.json` no se pueden leer o están corruptos al cargar, se usan los defaults, el archivo se renombra a `*.corrupt` (para no machacarlo en el siguiente guardado) y se anota en el log. Los fallos de escritura siguen la tabla de errores (sección 5).

## 9. Entrada de documentos

1. **Argumento de lanzamiento / segunda instancia:** sin asociaciones de archivo (sin instalador, `DIFERIDOS.md`); `tauri-plugin-single-instance` (la segunda instancia reenvía la ruta y se enruta como cualquier apertura, con origen = última lectora enfocada o, si ya no existe, cualquiera; las rutas relativas se resuelven contra el `cwd` que entrega el plugin; sin rutas, sólo enfoca esa ventana). Si la apertura de lanzamiento falla, se crea la ventana vacía y el aviso se muestra sobre ella; en segunda instancia, sobre la última ventana enfocada.
2. **Drag & drop de archivos:** se recibe en Rust (`WindowEvent::DragDrop`) y se enruta con la ventana del drop como origen. Sólo archivos: un directorio se ignora (log). Varios archivos se abren en el orden recibido, como una tanda. **Soltar texto no está soportado** *(verificar)*: en Windows, con el drop nativo de Tauri activo, WebView2 desactiva el drag & drop HTML5. El texto entra por Ctrl+V.
3. **Portapapeles al arrancar:** si no hay archivo de lanzamiento y el portapapeles tiene texto, crea el documento (comportamiento de `applicationDidFinishLaunching:`).
4. **Diálogo (Ctrl+O) y pegado (Ctrl+V):** sección 7.1; ambos en Rust (D10).
5. **Detección de formato:** empieza por `%PDF-` → PDF; por `{\rtf` → RTF; `PK\x03\x04` → zip con `META-INF/container.xml` → EPUB; cualquier otro zip → DOCX (sin `word/document.xml` → `Unsupported`; zip inválido → `Corrupt`); extensión `htm`/`html`/`xhtml` → HTML (así un `.txt` que empiece por `<html` no pierde sus etiquetas); el resto, texto plano.

### Importadores *(verificar versión y licencia de cada crate al montar)*

| Formato | Elección | Riesgo |
|---|---|---|
| HTML | `scraper` 0.29, decodificado con el `decode` de `plain.rs`. Recorrido del árbol: emiten bloque `p`, `h1`–`h6`, `li` (`p` dentro sigue siendo lista) y `tr` (dentro de una fila nada emite bloque); `blockquote` convierte en `Quote`; el resto es contenedor o en línea, y el texto suelto entre bloques forma un párrafo anónimo. Blancos ASCII colapsados (U+00A0 se conserva). Se ignoran `head`, `script`, `style`, `template`, `noscript`, `svg`, `math` y `[hidden]`. No se reinyecta HTML, así que no hace falta sanear | bajo |
| DOCX | `zip` 9 (sólo `deflate-flate2-zlib-rs`) + `quick-xml` 0.42. `styles.xml`: títulos y citas por `w:name` (inglés aunque Word esté en español). `numbering.xml`: `bullet` → `•`, `none` → nada, otro → decimal por (`numId`, `ilvl`) con reinicio de niveles más profundos; sin formato → `•`. `document.xml`: `w:b`/`w:i`/`w:u`, `w:t`, `w:tab`, `w:br`/`w:cr`; se ignoran `w:del`, `w:instrText`, `mc:Fallback`, `w:drawing`, `w:pict`, `w:txbxContent`. Runs contiguos del mismo estilo se fusionan | bajo |
| RTF | **Lector propio** (`rtf.rs`, sin RichEdit: `DECISIONES.md`). Un recorrido de bytes con pila de estado por grupo. `\par`/`\sect`/`\page` cierran bloque, `\line` lo parte, `\b`/`\i`/`\ul`/`\plain`, `\outlinelevelN` → título, `{\listtext}`/`{\pntext}` → marcador de lista (`"• "` si no tiene letras ni dígitos), tablas por `\cell`/`\row`. Texto por `\ansicpgN` (`codepage` + `encoding_rs`), `\uN` con salto de `\ucN` (nunca cruza palabras de control ni llaves). Se omiten `{\*…}`, tablas de fuentes/colores/estilos, `info`, imágenes, objetos, cabeceras/pies, notas, `fldinst` y `\v`. Nunca falla: llaves desequilibradas o truncado conservan el texto. Fuera: `\fcharset` por fuente | bajo |
| PDF | **`pdfium-render` 0.9.5** (`thread_safe`, `pdfium_latest`) + **`pdfium.dll` x64** de `bblanchon/pdfium-binaries` `chromium/8125`, versionada en `src-tauri/pdfium/` y embebida en el ejecutable (`include_bytes!`); se extrae una vez a `%TEMP%\miso-pdfium-8125.dll` (si falta o cambia de tamaño) y se carga de ahí. Una instancia por proceso (`OnceLock`). Párrafos reconstruidos por página: líneas por los `\r\n` generados (el guion de fin de línea llega como U+0002 con `is_hyphen` y también parte línea), estadísticas por tramo entre subidas de columna (alto, márgenes, hueco) y corte por línea corta (sólo si la primera palabra de la línea siguiente, medida con sus glifos, cabía en el hueco: texto sin justificar), hueco o sangría; guion + minúscula se une. Entre páginas, el último párrafo continúa en el primero si acaba en guion, o si no acaba en fin de frase (`. ? ! … » " ”`) y el siguiente empieza en minúscula (el guion final llega sin `is_hyphen`). Todo `Paragraph`. Sin texto → `Unsupported`. Fuera: títulos, estilos, cabeceras/pies | medio |
| EPUB | `epub.rs` sobre zip + quick-xml, sin `mimetype` obligatorio. `container.xml` → primer `rootfile` → OPF; se recorre el `spine` entero en orden (también `linear="no"`), sólo ítems `application/xhtml+xml` o `text/html`; un `idref` roto, un `href` ausente o un capítulo no decodificable se saltan (`log::warn!`). Cada capítulo pasa por `plain::decode` y por el importador HTML tras cortar antes de `<body` (sin distinguir mayúsculas). Cada ítem del spine que aporta bloques abre capítulo (`chapters`), salvo el primero; los vacíos no cuentan. Título = `dc:title` limpio (si falta, el nombre del archivo). Imágenes: `img[src]` y `<svg><image href>`, formatos jpg/png/gif/webp/svg; se omiten las externas, `data:`, las que salen de la raíz, no están en el zip, están cifradas o son de otro tipo. NCX y `nav` no se interpretan. DRM sólo se detecta (`encryption.xml`), no se descifra | medio |
| Texto/encoding | `encoding_rs` + `chardetng` | bajo |

## 10. Búsqueda

`textIndex` da el texto aplanado (`flatText`, bloques unidos por `\n`); `search.ts` encuentra coincidencias y devuelve rangos de carácter UTF-16. Es incremental: busca en cada tecla, con el término literal (escapado) y regex `giu`: no distingue mayúsculas, sí acentos; una coincidencia nunca cruza bloques. Se resalta sólo el resultado actual, con la **CSS Custom Highlight API** (no muta el DOM); la barra muestra «n de m». El primer resultado es el primero desde el ancla al abrir la barra; al refinar el término se busca desde el resultado activo, para no saltar. Al ir a un resultado no se desplaza si ya está visible; si no, queda centrado (`reveal`). WebView2 Evergreen se actualiza solo; la versión mínima fijada en el instalador es **Chromium 128** (cubre Highlight API, v105, y `caretPositionFromPoint`, v128). La integración con el pasteboard de búsqueda del sistema (`NSPasteboardNameFind`) y su temporizador de 1 s **no se replican**: no existen en Windows.

## 11. Seguridad

- CSP: `default-src 'self'; img-src 'self' miso-img: http://miso-img.localhost; connect-src ipc: http://ipc.localhost`; sin red a Internet. En `tauri dev` no se aplica. La comprobación con la app compilada (fase 7: `fetch` externo bloqueado, sin violaciones; Svelte aplica `style:` por CSSOM) es anterior a `img-src`: la de las imágenes EPUB queda en la verificación pendiente del titular.
- **Protocolo `miso-img`** (C7): registrado en `lib.rs`. Responde con los bytes de una imagen sólo si está en el `Doc` de la ventana que pide (`webview_label()`) y el path es su número; el resto → 404. No recibe rutas ni abre red (D6, D10).
- El HTML importado **nunca se inyecta como HTML**: Rust lo reduce a `Doc` y Svelte pinta nodos propios. Se elimina así la clase de problemas de XSS por documento.
- Ningún comando acepta rutas del frontend (D10): diálogos, drops y argumentos de lanzamiento se resuelven en Rust.
- Una sola capability (`reader`), sólo con permisos de core (eventos y ventana): sin `fs`, `dialog`, `clipboard`, red ni `opener` en el frontend. Los diálogos de archivo y de mensaje los abre Rust (D10, D11).
- Restricción de comandos propios: sin `AppManifest` ni comprobación de *label*. Sólo hay ventanas de lectura y todas usan todos los comandos (las prefs se editan desde el panel del lector).

## 12. Pruebas

- **Rust:** unit tests de cada importador contra `tests/fixtures` o documentos generados en el test (HTML y RTF en cadenas, DOCX y EPUB con `ZipWriter` en memoria (spine, capítulos, imágenes, cifrado, OPF roto, truncado), PDF creados con `pdfium-render` en el test; sólo `protegido.pdf` es fixture), incluyendo archivos truncados/corruptos (el error debe llegar como `AppError`, no como pánico; un importador que entra en pánico produce `Corrupt`); tests de prefs (validación) y posiciones (búsqueda por clave, poda, versión distinta → corrupto, flush con fallo de escritura).
- **Rust:** `file_id` (renombrar y mover dentro del volumen conserva la clave; copiar a otro volumen cae al hash; reescritura por editor cae a la `path`).
- **Rust:** enrutado (archivo ya abierto enfoca su ventana; ventana nueva recoge su `Doc` pendiente).
- **TS (vitest):** `columnWidth` (umbral √2 y múltiplo exacto en ventana normal), `textIndex` (paridad con Rust en UTF-16).
- **Layout (vitest browser mode vía Playwright con el Edge instalado, `tests/layout/`):** `vitest.config.ts` usa el `msedge.exe` de EdgeCore (el motor de WebView2) si existe y, si no, `channel: 'msedge'`. Sin descargar navegador. `pitch` medido, `columnScroll` (500 saltos sin error acumulado), ancla conservada tras cambiar fuente/ancho/tamaño. **Obligatorios**: jsdom no maqueta y sin ellos la fase 2 no se puede cerrar. Imágenes (`tests/layout/images.test.ts`, URLs `blob:` como `miso-img`, que llega de forma asíncrona): una alta cabe en la altura de columna y una ancha en el ancho; con imágenes antes del ancla, éste queda en la primera columna visible; una columna que empieza con imagen tiene como índice el inicio de su bloque; `measurePitch` correcto con una imagen primera; imagen rota → `hidden` y ancla conservada. Premisas de Chromium para la composición de libro (`multicol.test.ts`): margen truncado en salto no forzado, conservado tras salto forzado, sonda del ancla en sangría y en título centrado, blanco de alto 0; las medidas verticales son el borde del contenido (caja + `padding-block-start`). Composición de libro (`book.svelte.test.ts`): D9, separación, sangría, títulos centrados, sin hueco arriba, capítulos en columna nueva, ancla tras cada cambio de prefs, ancla sobre un blanco, búsqueda y paso.
- **Licencias:** `pnpm check:licenses` (primera etapa de `verify`) falla si un archivo de `src-tauri/pdfium/licenses/` no está en el manifiesto, si alguno no es UTF-8 o si los artefactos generados no coinciden con sus fuentes.
- **E2E:** opcional con `tauri-driver`; decidir cuando exista el flujo completo.

## 13. Plan por fases

| Fase | Entrega | Criterio de hecho |
|---|---|---|
| 0 | Scaffold Tauri 2 + Svelte 5 + TS, CI local (`check`, `test`, `test:layout`, `cargo clippy`) | la app abre una ventana vacía |
| 1 | Texto plano → `Doc` → columnas con CSS multicol; `pitch` medido; teclado por columnas y rueda; argumento de lanzamiento y Ctrl+O; enrutado con `Doc` pendiente; `AppError` (`Read`, `Unsupported`, `Internal`) + `report()`, log y panic hook | un `.txt` largo se lee en columnas y se navega; un archivo ilegible o no soportado muestra un aviso nativo y queda en el log |
| 2 ✅ | Ancla + reflow al redimensionar; ancho de viewport múltiplo exacto; ↑/↓ (= ←/→); prefs (fuente, ancho, insets, colores, interlineado) en un panel integrado en el lector y temas Claro/Oscuro (Ctrl+I); escala de fuente por ventana; Ctrl+, y Ctrl+= / − / 0; `hyphens: auto` comprobado (negativo) | cambiar cualquier pref o la escala conserva el primer carácter visible; 500 saltos sin error acumulado (tests de layout) |
| 3 ✅ | Persistencia de posición; portapapeles; drag & drop de archivos; ventana por documento; archivo ya abierto enfoca su ventana; single-instance | reabrir un archivo vuelve al punto exacto |
| 4 ✅ | Búsqueda incremental + resaltado | encontrar, saltar y centrar un resultado |
| 5 ✅ | Importadores HTML y DOCX; fullscreen con regla √2 | ambos formatos abren con estilos básicos |
| 6 ✅ | RTF (lector propio) y PDF (PDFium) | un RTF con negrita/cursiva abre con sus estilos; la mayoría de PDF de texto se lee con orden razonable |
| 7 ✅ | Exportar copia (texto, RTF) con Ctrl+S; bundle id, icono propio y ejecutable sin instalador (`pdfium.dll` embebida) | el `.exe` de *release* copiado a otra carpeta abre un PDF; la copia RTF abre en Word |
| EPUB ✅ | Importador EPUB con imágenes (`miso-img`), título del libro y DRM detectado; fuera de fases | un EPUB sin DRM abre en columnas con sus imágenes dentro de la columna; uno con capítulo cifrado muestra el aviso de protegido |
| 8 | Medición de rendimiento y segmentación si hace falta | documento de ~1 M de caracteres abre sin bloquear la UI |

## 14. Decisiones cerradas

Viven en `DECISIONES.md` (plataforma, PDF, nombre y bundle id, ventana).
