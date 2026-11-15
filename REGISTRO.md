# Registro

Cambios e intervenciones implementadas y verificadas. Brevedad ante todo: fecha + título, resumen en una o dos líneas, archivos tocados y estado.

## 2026-10-20 — Ajustes de la auditoría de ARCHITECTURE.md

Aplicadas las correcciones de `_AUDITORIA_ARCHITECTURE`: enrutado con `Doc` pendiente, `doc_key`/posición resueltos en Rust, E/S sólo en Rust (D10), tests de layout en Chromium, atajos y escala de fuente, respaldo por `path`, drop sólo de archivos, erratas. Decisiones cerradas movidas a `DECISIONES.md`.

- Archivos: `ARCHITECTURE.md`, `DECISIONES.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/.informes/_AUDITORIA_ARCHITECTURE_cumplido.md`.
- Estado: hecho (sólo documentación). Sin commit: el directorio no es un repositorio git.

## 2026-11-10 — Presentación de errores (D11)

Resuelto el diferido de fase 1: Rust registra y presenta los `AppError` con diálogo nativo vía `report()`; se elimina el evento `app-error`. Taxonomía por fases, log a archivo con panic hook, prefs/posiciones corruptas → defaults + `*.corrupt`, apertura de lanzamiento fallida → ventana vacía con aviso. Approach deliberado entre Opus y agy (Gemini).

Además, por decisión del titular, los tests de layout usan el Edge instalado (§12). Creado `_PLAN_FASE_0.md`. Fijados en documentos permanentes: versiones mayores (D5), compilar desde PowerShell (§4) e identificador provisional (`DIFERIDOS.md`).

- Archivos: `ARCHITECTURE.md` (§2 D11, §4, §5 Errores, §8, §9, §11, §12, §13), `DIFERIDOS.md`, `REGISTRO.md`, `_PLAN_FASE_0.md`.
- Estado: hecho (sólo documentación). Incluido en el commit de línea base `0046308`.

## 2026-11-11 — Fase 0: scaffold Tauri 2 + Svelte 5 + TS

Repositorio git, frontend mínimo (Svelte 5, Vite 8, TS 6), `src-tauri/` (Tauri 2, ventana `reader-0` vacía, capability `reader`, CSP, id provisional) y puertas locales (`pnpm verify`: check, test, test:layout en el motor de WebView2, clippy, cargo test). Implementado por agentes Haiku, revisado y ajustado por el orquestador. Verificado: ventana "Miso" abre y cierra limpia, Svelte monta, barra oscura con el sistema oscuro, `pnpm verify` verde y las puertas fallan ante errores provocados. Desviaciones: `.npmrc` hoisted (volumen exFAT); CSP no aplicada en `tauri dev` (diferido a fase 7); junction de Edge rechazada y sustituida por resolución en `vitest.config.ts`.

- Archivos: `.gitignore`, `.gitattributes`, `.npmrc`, `package.json`, `pnpm-lock.yaml`, `vite.config.ts`, `vitest.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html`, `src/*`, `src-tauri/*`, `tests/layout/multicol.test.ts`, `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_FASE_0_completado.md`.
- Estado: hecho y verificado.

## 2026-11-11 — Fase 1: texto plano en columnas, enrutado y errores

Importador de texto plano (BOM, UTF-8, chardetng, binario → `Unsupported`), tipos TS generados con `ts-rs`, enrutado con `Doc` pendiente y ventanas creadas por Rust, `open_dialog` (Ctrl+O) y argumento de lanzamiento, `AppError` + `report()`, log a archivo y panic hook; frontend con CSS multicol, `pitch` medido, teclado por columnas y rueda. Plan auditado por agy y Big Pickle (OpenCode); implementado por agentes Haiku, revisado y verificado por el orquestador. Verificado: `pnpm verify` verde (Rust 21, unit 21, layout 4), negativas de las puertas, y en `tauri dev` errores al log con ruta completa y título de ventana = archivo. Pendiente de confirmación visual del titular: aviso sobre la ventana, columnas y navegación.

- Archivos: `.cargo/config.toml`, `package.json`, `pnpm-lock.yaml`, `vitest.config.ts`, `src-tauri/{Cargo.toml,Cargo.lock,tauri.conf.json}`, `src-tauri/src/{lib,error,model,doc_windows,commands}.rs`, `src-tauri/src/import/*`, `src/App.svelte`, `src/components/*`, `src/lib/{ipc,layout,nav}/*`, `tests/fixtures/*`, `tests/layout/columns.test.ts`, `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_FASE_1_completado.md`, `.historial/.informes/_CIERRE_FASE_0_cumplido.md`.
- Estado: hecho y verificado. Confirmación visual del titular el 2026-11-12.

## 2026-11-12 — Ventana y tipografía ajustadas a Tofu

Ventana inicial 1150×640 (3 columnas por defecto, más apaisada). Equivalencia con AppKit en `Columns.svelte`: 5 px de `lineFragmentPadding` por lado sin cambiar el paso (381.6 px) e interlineado `calc(1.1 × 1lh)` sobre el alto natural de la fuente (15.4 → 20.9 px). Comparado a ojo contra capturas de Tofu por el titular; el trazo más fino en Windows es del renderizado del sistema y se deja así.

- Archivos: `src-tauri/src/doc_windows.rs`, `src/components/Columns.svelte`, `ARCHITECTURE.md` (§6), `REGISTRO.md`.
- Estado: hecho. `pnpm lint:rust` y `pnpm test:layout` en verde; interlineado medido en el motor de Edge; aspecto confirmado por el titular.

## 2026-11-12 — Fase 2: ancla, viewport exacto, prefs y atajos

Prefs persistentes en Rust (`prefs.rs`: recorte por H4, `prefs.json` atómico, `.corrupt` al fallar la lectura, `Persist` con aviso una vez por sesión, `prefs-changed` y color de fondo nativo); comandos `get/set/reset_prefs` (estos dos, sólo desde la ventana `prefs`) y `open_prefs`; `os_message` sin ` (os error N)`. Frontend: prefs por *props* a `Columns`, viewport de N pasos exactos (`fitViewport`), ancla deseada con restauración tras reflow, ↑/↓ = ←/→, escala por ventana (Ctrl+= / − / 0), Ctrl+, y ventana `Preferences` con temas Claro/Oscuro. `hyphens: auto` dio negativo: sin pref. Orquestado por Opus; P3, la parte pura de P6 y `Preferences.svelte` por agentes Haiku; ancla, viewport y tests por el orquestador.

Desviaciones argumentadas: `serde_json` pasa de dev a dependencia normal (ya estaba en el lock); `--vp-w` se escribe de forma imperativa (con `$state` el `$effect` medía el `pitch` con el viewport a 0 px y `goTo` quedaba desplazado); la guarda del ancla compara `scrollLeft` con la posición restaurada en vez de usar el indicador `restoring`, que una segunda restauración (resize + efecto) apagaba antes de su `scrollend`; `capabilities/prefs.json` con sólo `core:window:allow-close` (Esc cierra Preferencias); los botones Claro/Oscuro van en fila propia. Rechazada la del agente de P3 que reactivaba el aviso de `Persist` tras un guardado correcto (§5: una vez por sesión).

Verificado: `pnpm verify` verde (Rust 33, unit 46, layout 15: 7 cambios de pref/escala/ancho conservan el ancla, 20 cambios alternos sin deriva, 500 saltos con error < 1 px); negativas: sin la guarda fallan 6 tests de ancla; sin recorte superior de `column_width` falla `test:rust`. En `tauri dev`: Ctrl+, abre «Preferencias» y un segundo Ctrl+, no la duplica; cambiar el tamaño escribe `prefs.json` (UTF-8, sin nulos) y 99 se recorta a 48; Esc la cierra; log sin errores.

- Archivos: `src-tauri/Cargo.toml`, `src-tauri/capabilities/prefs.json`, `src-tauri/src/{prefs,error,commands,doc_windows,lib}.rs`, `src/App.svelte`, `src/components/{Reader,Columns,EmptyState,Preferences}.svelte`, `src/lib/ipc/{commands,events}.ts`, `src/lib/ipc/types/Prefs.ts`, `src/lib/layout/{anchor,columnWidth,columnWidth.test}.ts`, `src/lib/nav/{columnScroll,keyboard,keyboard.test}.ts`, `tests/layout/{defaults,columns.test,anchor.svelte.test}.ts`, `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_FASE_2_completado.md`, `.historial/.informes/_CIERRE_FASE_1_cumplido.md`.
- Estado: hecho y verificado por agentes. Pendiente de confirmación del titular: zoom del WebView desactivado y comprobaciones visuales del paso 7 (`DIFERIDOS.md`).

## 2026-11-12 — Preferencias integradas, arranque sin espera y Ctrl+I

Tras la revisión visual del titular (ventana aparte fea y lenta), Preferencias pasa a ser un panel lateral superpuesto dentro del lector (Ctrl+, abre/cierra, Esc cierra; colores del sistema; sin IPC al abrir; las teclas de navegación no actúan dentro del panel). Se eliminan la ventana `prefs`, `open_prefs`, `capabilities/prefs.json` y la comprobación de *label* de `set_prefs`/`reset_prefs` (reabre H3, H6 y P7 con aprobación del titular). `create_reader` inyecta las prefs con `initialization_script`: el primer fotograma no espera a `getPrefs`, que pasa a reconciliar en paralelo con `take_pending_document`. Nuevo atajo Ctrl+I: alterna Claro/Oscuro según la luminancia del fondo y persiste (`theme.ts`, constantes compartidas con el panel).

- Archivos: `src-tauri/src/{commands,doc_windows,lib}.rs`, `src-tauri/capabilities/prefs.json` (borrado), `src/App.svelte`, `src/components/{Reader,Preferences}.svelte`, `src/lib/ipc/commands.ts`, `src/lib/theme.ts`, `src/lib/theme.test.ts`, `src/lib/nav/{keyboard,keyboard.test}.ts`, `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`.
- Estado: hecho. `pnpm verify` verde; panel y arranque confirmados por el titular. Ctrl+I verificado en unit; pendiente de probar en la app. La inyección no se midió aparte (si fallara, `getPrefs` la cubre sin diferencia visible).

## 2026-11-12 — Ctrl+rueda escala la fuente

Confirmación del titular de la fase 2: zoom del WebView desactivado, dos lectores en caliente, fondo oscuro sin destello y atajos del teclado español correctos. Faltaba Ctrl+rueda (fuera de alcance en fase 2): ahora escala la fuente de la ventana como Ctrl+= / Ctrl+− (`wheelZoom` puro; listener `passive: false` en `Reader`) y la rueda con Ctrl ya no desplaza columnas.

- Archivos: `src/lib/nav/{wheel,wheel.test}.ts`, `src/components/Reader.svelte`, `ARCHITECTURE.md` (§7.1), `DIFERIDOS.md`, `REGISTRO.md`.
- Estado: hecho. `pnpm verify` verde (Rust 33, unit 59, layout 15). Pendiente de probar en la app por el titular.

## 2026-11-12 — Fase 3: posición persistente, portapapeles, drop y una sola instancia

`store.rs` común (prefs y posiciones); `DocKey` (`file_id` por `FileIdInfo`, FNV-1a de tamaño + 64 KB, ruta canónica); enrutado con regla 1 (misma ruta o mismo `file_id` → enfoca) y toda entrega por el pendiente + `document-ready` (H1); `open_paths` único para arranque, Ctrl+O, drop y segunda instancia, con errores de tanda en un diálogo («Y N más.»); `positions.json` versionado con tope de 1000, debounce de 5 s en un hilo y un solo `Mutex`, flush al cerrar ventana y app; `set_position(char_index)` sin clave en el frontend (H2); portapapeles con `arboard` (Ctrl+V y al arrancar sin rutas); `tauri-plugin-single-instance` con origen = última lectora enfocada. Frontend: Ctrl+V (pega en los campos del panel), ancla inicial desde `initial_char_index` y `onanchor` → `set_position` sólo con scroll del usuario. Orquestado por Opus; P2–P8 por agentes Haiku con diseño fijado y revisión de cada diff.

Desviaciones argumentadas: `import()` sólo en tests (sin consumidor); ventana destino desaparecida al enfocar/entregar → `Internal`; `last_reader` con `try_state` (sin pánico en el callback FFI); tests de `store` con tipo propio; `canonicalize` deja el prefijo `\\?\` en `path` (coherente en todas las vías).

Verificado: `pnpm verify` verde (Rust 77, unit 63, layout 17). Negativa: sin reescribir la clave en `set` falla `set_reescribe_la_clave_sin_anadir`. En la app (binario de desarrollo + Vite): tres argumentos (uno bueno, dos inexistentes) → una ventana y los dos errores en una tanda; AvPág guarda `positions.json` a los 5 s (UTF-8, sin nulos, con `file_id`); reabrir registra el mismo índice (17337); segunda instancia: mismo archivo enfoca sin ventana nueva, relativo resuelto con el `cwd` del plugin y abierto en la primera, sin rutas sólo enfoca; un solo proceso; arranque sin rutas con texto copiado abre «Portapapeles». No verificado: Ctrl+V en la app (las pulsaciones con Ctrl sintéticas no llegaban a WebView2).

- Archivos: `src-tauri/{Cargo.toml,Cargo.lock}`, `src-tauri/src/{lib,commands,doc_windows,error,model,prefs,store,file_id,positions,clipboard}.rs`, `src-tauri/src/import/{mod,plain}.rs`, `src/lib/ipc/{commands,events}.ts`, `src/lib/ipc/types/Doc.ts`, `src/lib/nav/{keyboard,keyboard.test,columnScroll}.ts`, `src/lib/layout/textIndex.test.ts`, `src/components/{Reader,Columns}.svelte`, `tests/fixtures/unicode.doc.json`, `tests/layout/{defaults,anchor.svelte.test}.ts`, `ARCHITECTURE.md` (§4, §5, §8, §9, §12, §13), `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_FASE_3_completado.md`.
- Estado: hecho y verificado por el orquestador. Confirmado por el titular el 2026-11-12: posición exacta al reabrir, Ctrl+V (vacía, llena, campo del panel), drop (uno, varios, carpeta, ya abierto) y segunda instancia (la ventana pasa al frente).

## 2026-11-12 — Fase 4: búsqueda incremental y resaltado

Todo en el frontend: `flatText`/`blockAt` en `textIndex`, `search.ts` (término literal, regex `giu`, sin cruzar bloques), `highlight.ts` (sólo el resultado actual con la Highlight API), `reveal` en `columnScroll` (no se mueve si ya se ve; si no, centra), `FindBar` flotante con «n de m», y Ctrl+F / F3 / Mayús+F3 / Intro / Esc en `keyboard` y `Reader`. Plan auditado por agy y Big Pickle (OpenCode); P2–P4 implementados por DeepSeek v4.1 Flash (APInex) y revisados por el orquestador.

Desviaciones argumentadas: bucle con `exec` en vez de `matchAll` (target es2017); `nodeAt` exportado de `anchor.ts` y reutilizado. Corregido en revisión: la condición «ya visible» de `reveal` era errónea y su test no lo detectaba (usaba la columna 0); reescrito con la última columna visible. Incidente: en P4 el agente agotó la cuota y antes deshizo el trabajo con `git stash` + `drop`, editó la documentación y archivó el plan; recuperado desde el stash descartado y descartados sus cambios de documentación.

Verificado: `pnpm verify` verde (Rust 77, unit 82, layout 22). Negativas: sin la comprobación «ya visible» falla su test de layout; sin la vuelta de `firstAtOrAfter` falla su test unitario.

- Archivos: `src/lib/find/{search,search.test,highlight}.ts`, `src/lib/layout/{textIndex,textIndex.test,anchor}.ts`, `src/lib/nav/{columnScroll,keyboard,keyboard.test}.ts`, `src/components/{FindBar,Columns,Reader}.svelte`, `tests/layout/anchor.svelte.test.ts`, `ARCHITECTURE.md` (§4, §7.1, §10, §13), `REGISTRO.md`, `.historial/_PLAN_FASE_4_completado.md`.
- Estado: hecho y verificado. Confirmado por el titular el 2026-11-12: Ctrl+F y F3 abren la barra propia (no la de WebView2) y todas las comprobaciones de la lista pasan.

## 2026-11-12 — Texto plano cortado a ancho fijo se lee en párrafos

Defecto de fase 1 (G5): un `.txt` cortado a ~70 caracteres (Gutenberg) se pintaba con un párrafo por línea, como un poema. `plain::blocks` une el salto con un espacio cuando la primera palabra de la línea siguiente no cabía en el ancho del texto (percentil 95), la línea siguiente no está sangrada y eso ocurre en la mayoría de los saltos; con anchos < 50 o menos de 8 pares no une nada (versos, listas, textos cortos). La longitud no cambia: posiciones guardadas y búsqueda siguen válidas.

Verificado: `pnpm verify` verde (Rust 81, unit 82, layout 22). Negativa: sin la regla de mayoría falla `poema_y_parrafos_por_linea_no_se_unen`; el poema de versos regulares reveló y fijó el ancho mínimo.

- Archivos: `src-tauri/src/import/plain.rs`, `ARCHITECTURE.md` (§5), `REGISTRO.md`.
- Estado: hecho y verificado. Confirmado por el titular el 2026-11-12.

## 2026-11-12 — Literata 400 como fuente por defecto

Por decisión del titular, la fuente por defecto pasa de Palatino Linotype a Literata 400. Va empaquetada en la app (OFL, `@fontsource/literata` 5.3.0, latín y latín extendido en `src/assets/fonts/`, sin CDN en tiempo de ejecución) con `font-display: block`; `main.ts` la carga antes de montar para que el paso de columna se mida con ella (si falla, monta igual). Primera en la lista del panel. Las prefs ya guardadas no cambian.

Desviación de test: el centrado de «coincidencia lejana» tolera 1 px a la izquierda, como ya hacía su cálculo de columna (`scrollLeft` entero con paso fraccionario; con Palatino el redondeo caía a favor).

Verificado: `pnpm verify` verde (Rust 81, unit 82, layout 23). Negativa: con la ruta de la fuente rota falla el test de carga de Literata.

- Archivos: `src/fonts.css`, `src/assets/fonts/{literata-latin-400-normal,literata-latin-ext-400-normal}.woff2`, `src/assets/fonts/OFL.txt`, `src/main.ts`, `src/components/Preferences.svelte`, `src-tauri/src/prefs.rs`, `tests/layout/{defaults,columns.test,anchor.svelte.test}.ts`, `ARCHITECTURE.md` (§8), `REGISTRO.md`.
- Estado: hecho y verificado por el orquestador. Sustituida el 2026-11-13 por la variable (asiento siguiente).

## 2026-11-13 — Partición de palabras y trazo más grueso

Por decisión del titular («más literario», «grueso y nítido como en mac»), se reabre la regla de fase 2 sin partición. `hyphenate.ts` inserta guiones blandos al pintar (paquete `hyphen` 1.14.1, ISC, patrones incluidos) en palabras de 6+ letras, con idioma detectado por palabras frecuentes (`en`/`es`; si no es claro, no parte); `lang` de las columnas = idioma detectado. Los guiones sólo viven en el DOM: `anchor.ts` traduce entre DOM e índice (`modelOffset`/`domOffset`), Ctrl+C los quita y el importador elimina los U+00AD del original. Literata pasa a la variable (peso + tamaño óptico) y el lector pinta a 450.

Defectos encontrados por los tests y corregidos: el guion visible es otro rectángulo en la misma línea y `measurePitch` lo tomaba por columna nueva (paso de 36 px en vez de 381); `charRect` usaba el primer rectángulo, que tras una palabra partida es el guion de la línea anterior, y el ancla se restauraba una columna atrás. Oráculos de test ajustados por la misma causa (justificado): `textRects`, `charLeft` y el de 500 saltos saltan el guion; `rangeOf` compara sin guiones blandos.

Verificado: `pnpm verify` verde (Rust 81, unit 89, layout 24). Negativas: con `charRect` en el primer rectángulo fallan 3 tests de ancla (incluido el nuevo «primer carácter visible de cada columna»); sin la traducción de índices fallan 4.

- Archivos: `package.json`, `pnpm-lock.yaml`, `src/lib/layout/{hyphenate,hyphenate.test,anchor}.ts`, `src/lib/nav/columnScroll.ts`, `src/components/Columns.svelte`, `src/fonts.css`, `src/assets/fonts/*`, `src-tauri/src/import/plain.rs`, `tests/layout/{defaults,anchor.svelte.test}.ts`, `ARCHITECTURE.md` (§5, §6, §8), `DIFERIDOS.md`, `REGISTRO.md`.
- Estado: hecho y verificado por el orquestador; pendiente de confirmación visual del titular.

## 2026-11-13 — Línea en blanco a media línea

El titular veía muy grande el hueco entre párrafos: la línea en blanco del `.txt` se pintaba con una línea entera. Ahora un bloque vacío (`p.blank`) mide 0.5lh; el texto y los índices no cambian.

Verificado: `pnpm verify` verde (Rust 81, unit 89, layout 25, con test nuevo del alto de la línea en blanco).

- Archivos: `src/components/Columns.svelte`, `tests/layout/columns.test.ts`, `ARCHITECTURE.md` (§6), `REGISTRO.md`.
- Estado: hecho y verificado por el orquestador; pendiente de confirmación visual del titular.

## 2026-11-13 — Fase 5: HTML, DOCX y pantalla completa

HTML (`scraper`) y DOCX (`zip` + `quick-xml`: títulos por `styles.xml`, listas por `numbering.xml`, tablas aplanadas a una fila por párrafo) se abren con estructura y negrita/cursiva/subrayado, pintados con la fuente y los colores de prefs (C2). Error `Corrupt` para zip/XML dañado o pánico del importador. F11 alterna pantalla completa con la regla √2 de Tofu y Esc sale. `measurePitch` pasa a medir las cajas de los `<p>`, y las sangrías se hacen con `padding` (con citas o listas, medir líneas tomaba la sangría por el paso: agy en la auditoría). Plan auditado por agy y OpenCode (Big Pickle) e implementado por agentes Haiku con revisión del orquestador. La revisión corrigió `<li><p>` y `<td><p>` en HTML, los cuadros de texto de DOCX y dos tests que no probaban lo que decían. Detalle: `.historial/_PLAN_FASE_5_completado.md`.

Verificado: `pnpm verify` verde (Rust 118, unit 98, layout 32). Negativas: con el `measurePitch` anterior fallan el test del paso y el del ancla; sin el interlineado propio de los títulos falla el suyo; sin el reinicio de contadores falla el de la numeración.

- Archivos: `src-tauri/{Cargo.toml,Cargo.lock}`, `src-tauri/capabilities/reader.json`, `src-tauri/src/{model,error}.rs`, `src-tauri/src/import/{mod,plain,html,docx}.rs`, `src/lib/ipc/types/{Block,Run}.ts`, `src/components/{Columns,Reader}.svelte`, `src/lib/layout/{columnWidth,columnWidth.test}.ts`, `src/lib/nav/{columnScroll,keyboard,keyboard.test}.ts`, `tests/layout/{columns.test,anchor.svelte.test}.ts`, `ARCHITECTURE.md` (§4, §5, §6, §7.1, §8, §9, §12, §13), `DIFERIDOS.md`, `REGISTRO.md`.
- Estado: hecho y verificado por el orquestador; pendiente de la comprobación del titular en la app (P6: un `.docx` real en español, una página `.html`, un `.docx` truncado y F11/Esc).

## 2026-11-13 — Fase 6: RTF y PDF

RTF con un lector propio (`rtf.rs`, sin RichEdit: reabre §9 con aprobación del titular). Abre con títulos, listas, tablas, negrita/cursiva/subrayado y tildes (`\ansicpgN`, `\uN` con `\ucN`), y nunca falla. PDF con `pdfium-render` 0.9.5 y `pdfium.dll` (`chromium/8125`) versionada en `src-tauri/pdfium/`. Los párrafos se reconstruyen por geometría, página a página. Un PDF con contraseña da el aviso `Protected`; uno sin texto, `Unsupported`. La detección empieza por el contenido (`%PDF-`, `{\rtf`) y Ctrl+O ofrece «Documentos». La fusión de runs de DOCX pasa a `import::push_run`, compartida con RTF. Plan auditado por agy y OpenCode (Big Pickle), implementado por agentes Haiku y revisado por el orquestador. La revisión corrigió el salto de `\ucN` a través de llaves, un `\line` dentro de un elemento de lista, el guion de fin de línea de PDFium (U+0002 con `is_hyphen`; desvío de C7 pendiente de ratificar) y un test de columnas que no probaba lo que pedía el plan. Detalle: `.historial/_PLAN_FASE_6_completado.md`.

Verificado: `pnpm verify` verde (Rust 158, unit 98, layout 32). Negativas: sin el salto de `\ucN` fallan 3 tests; sin la regla de línea corta, 2. En `tauri dev`, un `.rtf` y un `.pdf` abren sin errores y la DLL carga desde `src-tauri/pdfium`.

- Archivos: `.gitattributes`, `src-tauri/{Cargo.toml,Cargo.lock}`, `src-tauri/pdfium/{pdfium.dll,LICENSE}`, `src-tauri/src/{error,commands}.rs`, `src-tauri/src/import/{mod,docx,rtf,pdf}.rs`, `tests/fixtures/protegido.pdf`, `ARCHITECTURE.md` (§4, §5, §9, §12, §13), `DECISIONES.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_FASE_6_completado.md`.
- Estado: hecho y verificado por el orquestador. Pendiente del titular: confirmar en la app (un `.rtf` de Word/WordPad con títulos, negritas, listas y tildes; un PDF de texto en párrafos y en orden; un PDF con contraseña y uno escaneado con su aviso; Ctrl+O con «Documentos»; reabrir un PDF vuelve al punto) y ratificar el desvío del guion en C7.

## 2026-11-13 — PDF: párrafos que cruzan página

Causa confirmada: el último renglón de cada página cerraba siempre el párrafo (C7). Ahora el último párrafo de una página continúa en el primero de la siguiente si no acaba en puntuación final (`. ? ! … » " ” : ;`) y el siguiente empieza en minúscula; el guion final se une sin espacio (PDFium no marca `is_hyphen` en el último renglón de la página). La raya (U+2014) llega intacta; el helper `ancho` de los tests contaba bytes y se corrige a caracteres.

Verificado: `pnpm verify` verde (Rust 160, unit 98, layout 32). Tests nuevos: párrafo que cruza, corte legítimo (punto final; mayúscula tras línea sin punto) y guion entre páginas.

- Archivos: `src-tauri/src/import/pdf.rs`, `ARCHITECTURE.md` (§9), `DIFERIDOS.md`, `REGISTRO.md`.
- Estado: hecho. Pendiente del titular: confirmar con el PDF real.

## 2026-11-13 — PDF: líneas cortas en texto sin justificar

Causa confirmada con `El Pensamiento del Afuera.pdf`: el texto va en bandera (bordes derechos entre 450 y 510 pt) y la regla de línea corta de C7 (`right < p90 − 2·alto`) suponía texto justificado; cortaba a mitad de frase 136 líneas (los casos del titular acaban en ~468–485 pt). La unión entre páginas no era la causa. Ahora una línea corta sólo cierra párrafo si la primera palabra de la siguiente (más su espacio, medida con los glifos) cabía en el hueco. Además, entre páginas se une tras `:`/`;` + minúscula y tras guion final + mayúscula («Le très-»/«Haut»). El `-5` no es número de página (el PDF no los tiene): es el número de la sección cuyo título abre la página siguiente; se conserva (anotado en DIFERIDOS).

Verificado: con el PDF real, 209 → 65 bloques; los 11 cortes señalados quedan dentro de un bloque; los cortes restantes son títulos, números de sección, finales de párrafo con hueco vertical y el subtítulo «-¡OH, AMIGOS!». `pnpm verify` verde (Rust 162, unit 98, layout 32). Tests nuevos (negativas comprobadas): línea corta sin justificar cuya palabra siguiente no cabía; dos puntos y guion al final de página.

- Archivos: `src-tauri/src/import/pdf.rs`, `ARCHITECTURE.md` (§9), `DIFERIDOS.md`, `REGISTRO.md`.
- Estado: hecho. Pendiente del titular: confirmar en la app.

## 2026-11-14 — Fase 7: exportar copia y ejecutable propio

Ctrl+S guarda una copia del documento en texto (UTF-8, CRLF) o RTF (ASCII, `\pard\plain` por bloque, `\uN?` fuera de ASCII); el formato sale de la extensión elegida en el diálogo. Rust conserva el `Doc` de cada ventana; nuevo error `Write`. Bundle id `com.mutant77.miso`, icono desde `assets/miso.svg`, sin instalador; `pdfium.dll` embebida en el `.exe` y extraída a `%TEMP%`. Plan auditado por agy, Big Pickle (OpenCode) y Nemotron 3 Ultra (pi); implementado por agentes Haiku, revisado por el orquestador.

Verificado: `pnpm verify` verde (Rust 168, unit 102, layout 32); negativa: sin `\plain` fallan 3 tests de `export`; ida y vuelta RTF con `import::rtf`. Titular: Ctrl+S en ambos formatos, `.rtf` en Word, `.exe` de *release* copiado (icono, PDF) y CSP bloqueando `fetch` externo. Desvíos en el plan (carpeta de datos creada al primer guardado, Ctrl+Shift+S, `64x64.png`).

- Archivos: `src-tauri/src/{export,commands,doc_windows,error,lib}.rs`, `src-tauri/src/import/pdf.rs`, `src-tauri/tauri.conf.json`, `src-tauri/icons/*`, `assets/miso.svg`, `src/lib/nav/keyboard{,.test}.ts`, `src/lib/ipc/commands.ts`, `src/components/Reader.svelte`, `ARCHITECTURE.md`, `DECISIONES.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_FASE_7_completado.md`.
- Estado: hecho y verificado.

## 2026-11-14 — EPUB con imágenes

EPUB sin DRM: capítulos en el orden del spine con el importador HTML (corte antes de `<body`), título de ventana = `dc:title`, imágenes como bloque `Image { id }` de 0 unidades servidas por el protocolo `miso-img` (sólo a la ventana que las pide; CSP con `img-src`). Capítulo cifrado → `Protected` («contraseña o DRM»); sólo fuentes ofuscadas → abre. Helpers de zip/XML compartidos con DOCX en `import/mod.rs`; `save_copy` propone el nombre del archivo. Implementado por agentes Haiku (P3 y P5 en paralelo), revisado por el orquestador.

Verificado: `pnpm verify` verde (Rust 209, unit 103, layout 38; layout ×3 estable); negativas de cada paso (`flush()`, spine, `encryption.xml` ×2, otra ventana, `restore()` tras `decode()`, `max-height`, imagen = 1 unidad). Humo con el importador real: *La acción humana* (4.116 bloques, 4/4 imágenes, título del libro) y EPUB de prueba con capítulo cifrado → `Protected`. Titular: texto de EPUB reales en `tauri dev`. Pendientes en `DIFERIDOS.md`: CSP en el ejecutable, `image/jpg`, imágenes en DOCX/HTML y composición de libro (plan aparte). Desvíos en el plan (`Promise.all` + `.catch`, `img:not([hidden])`, `blob:` en tests, `href` con `/` inicial).

- Archivos: `src-tauri/src/{model,export,clipboard,commands,doc_windows,error,lib}.rs`, `src-tauri/src/import/{mod,epub,html,docx,rtf}.rs`, `src-tauri/{Cargo.toml,Cargo.lock,tauri.conf.json}`, `src/lib/ipc/{commands.ts,types/Block.ts}`, `src/lib/layout/{textIndex,textIndex.test,hyphenate}.ts`, `src/components/Columns.svelte`, `tests/layout/{columns,images}.test.ts`, `ARCHITECTURE.md`, `DECISIONES.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_EPUB_completado.md`.
- Estado: hecho y verificado; pendiente del titular la CSP en el ejecutable.

## 2026-11-14 — Composición de libro

Todo documento que no es texto plano ni portapapeles se compone como libro: títulos centrados, sangría de primera línea (salvo tras título, imagen o inicio de capítulo), media línea entre párrafos sin hueco arriba de la columna, líneas en blanco de alto 0 y, en EPUB, cada ítem del spine con bloques en columna nueva. `Doc` gana `composition` (`Text`/`Book`) y `chapters` (índices de bloque); el índice de carácter, el ancla, la búsqueda y la exportación no cambian. Implementado por agentes Haiku (P2, P3, P4 en dos mitades), revisado por el orquestador.

Verificado: `pnpm verify` verde (Rust 215, unit 109, layout 77; layout ×4 estable); negativas de cada paso (padding en lugar de margen, capítulos vacíos, `Book` en texto plano, copia en `from_bytes`, `prev` sin saltar blancos, sin `break-before`, margen en `.chapter`, `display: none`, `::before`). Humo con el importador real: *La acción humana* (95 capítulos, «INTRODUCCIÓN» abre capítulo tras la firma del anterior) y *Libro del desasosiego* (517 capítulos, uno por fragmento). Titular: revisión visual en `tauri dev` conforme. Desvíos en el plan (medida vertical = borde del contenido, contenedor de 600 px, `book.svelte.test.ts`, blancos entre párrafos en el libro de prueba). Pendientes en `DIFERIDOS.md`: saltos de página DOCX/RTF como capítulo y títulos por clase.

- Archivos: `src-tauri/src/{model,clipboard,doc_windows,export}.rs`, `src-tauri/src/import/{mod,epub}.rs`, `src/lib/ipc/types/{Doc,Composition}.ts`, `src/lib/layout/{composition,composition.test,textIndex.test,hyphenate.test}.ts`, `src/components/Columns.svelte`, `tests/fixtures/unicode.doc.json`, `tests/layout/{defaults,multicol.test,anchor.svelte.test,columns.test,images.test,book.svelte.test}.ts`, `ARCHITECTURE.md`, `DECISIONES.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/_PLAN_COMPOSICION_completado.md`.
- Estado: hecho y verificado.
## 2026-11-14 — Abrir con archivo sin ventana vacía previa

Al lanzar Miso con un archivo, Rust lo importa antes de crear la ventana, que nace con el `Doc` pendiente y su título; si ninguno abre, `reader-0` vacía con el aviso de siempre. El frontend no muestra `EmptyState` (Ctrl+O) hasta resolver el primer `take_pending_document`. Verificado: clippy, Rust 215, `svelte-check` y unit 109 en verde. Pendiente del titular: comprobación con doble clic sobre un libro.

- Archivos: `src-tauri/src/lib.rs`, `src/components/Reader.svelte`, `ARCHITECTURE.md`, `REGISTRO.md`.
- Estado: hecho; pendiente de confirmación visual.

## 2026-11-15 — Fuentes del sistema en Preferencias

Nuevo comando `list_fonts`: Rust enumera las familias instaladas con DirectWrite (nombre en inglés si existe), descarta las verticales (`@`) y las que la validación de prefs rechazaría, y las ordena. El desplegable de «Fuente» muestra Literata (embebida) y después esa lista, en lugar de seis sugerencias fijas. El campo sigue siendo libre. Verificado: `pnpm verify` en verde (Rust 216 con test de enumeración en Windows, unit 109, layout 77). Pendiente del titular: abrir Preferencias y elegir una fuente del sistema.

- Archivos: `src-tauri/Cargo.toml`, `src-tauri/src/{fonts,commands,lib,prefs}.rs`, `src/lib/ipc/commands.ts`, `src/components/Preferences.svelte`, `ARCHITECTURE.md`, `REGISTRO.md`.
- Estado: hecho; pendiente de confirmación visual.

## 2026-11-15 — Interfaz multilingüe

11 idiomas de interfaz (inglés por defecto) con catálogo único en `locales/`, compartido por Rust (`i18n.rs`) y Svelte (`i18n.svelte.ts`). Autodetección por la lista de idiomas del sistema (`sys-locale`) y selector «Idioma» en Preferencias (`language` en prefs, cambio en vivo en todas las ventanas; `Restablecer` lo conserva). Errores, filtros de diálogo y título del portapapeles traducidos en Rust. Literata con cirílico embebido. Traducciones y descarga de la fuente por agentes Haiku, revisadas por el orquestador; plan auditado por DeepSeek 4.1 Flash. Verificado: `pnpm verify` en verde (Rust 226, unit 114, layout 77).

- Archivos: `locales/*.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/{i18n,lib,prefs,error,commands,clipboard,doc_windows}.rs`, `src/lib/i18n.svelte.ts`, `src/lib/i18n.test.ts`, `src/lib/ipc/types/Prefs.ts`, `src/components/{Preferences,FindBar,EmptyState,Reader}.svelte`, `src/main.ts`, `src/fonts.css`, `src/assets/fonts/literata-cyrillic{,-ext}-opsz-normal.woff2`, `index.html`, `tsconfig.json`, `vitest.config.ts`, `tests/layout/defaults.ts`, `ARCHITECTURE.md`, `DECISIONES.md`, `REGISTRO.md`, `.historial/_PLAN_I18N_completado.md`.
- Estado: hecho; pendiente de confirmación visual (idioma del sistema, cambio en vivo, anchura de etiquetas en de/fi).

## 2026-11-15 — Selector de fuentes del sistema

Las fuentes instaladas no aparecían en Preferencias: `list_fonts` las devolvía, pero el `<datalist>` de WebView2 filtra por el texto del campo («Literata») y sólo mostraba esa. El campo pasa a `<select>` con Literata primero, la familia actual aunque no esté instalada y las del sistema por orden alfabético; el foco inicial vuelve a la fuente. Verificado: `svelte-check` y unit 114 en verde. Pendiente del titular: abrir Preferencias y elegir una fuente del sistema.

- Archivos: `src/components/Preferences.svelte`, `ARCHITECTURE.md`, `REGISTRO.md`.
- Estado: hecho; pendiente de confirmación visual.

## 2026-11-15 — Preparación para GitHub

Revisión previa a publicar: sin secretos ni rutas personales en el código. `.gitignore` excluye ejecutables (`miso.exe`) y `.claude/`; README nuevo con requisitos, comandos y terceros. Sin push ni remoto.

- Archivos: `.gitignore`, `README.md`, `REGISTRO.md`.
- Estado: hecho; licencia pendiente de elegir por el titular.

## 2026-11-15 — README en inglés con cabecera y atajos

README reescrito en inglés y con más rigor: cabecera centrada con el logo (`assets/miso.svg`), distintivos de stack, sección de motivos, tabla completa de atajos extraída de `keyboard.ts`, requisitos, scripts de desarrollo, estructura y terceros. Sin cambios de código.

- Archivos: `README.md`, `REGISTRO.md`.
- Estado: hecho.

## 2026-11-15 — Publicación del repositorio en GitHub

`git init -b main`, commit `679f324` con 131 archivos y push a `origin` (`https://github.com/mutant-77/miso`), con seguimiento de `main`. `REGISTRO.md`, `DECISIONES.md`, `DIFERIDOS.md` y `.historial/` dejan de estar excluidos para versionar el histórico permanente. Se mantiene versionado `src-tauri/pdfium/pdfium.dll` (7,4 MB) por ser dependencia vendorizada del build.

- Archivos: `.gitignore`, `REGISTRO.md`.
- Estado: hecho y publicado.

## 2026-11-15 — Licencia MIT y avisos de terceros

Licencia MIT para Miso y cumplimiento de atribución de PDFium: añadida la carpeta `licenses/` del release `chromium/8125` de `pdfium-binaries` (18 textos, el DLL verificado por hash como el mismo binario) y `THIRD_PARTY_NOTICES.md` con el inventario. Todas las licencias son permisivas (BSD, MIT, Apache-2.0, FTL, Unicode); ninguna impone copyleft sobre el código propio.

- Archivos: `LICENSE`, `THIRD_PARTY_NOTICES.md`, `src-tauri/pdfium/licenses/*`, `README.md`, `ARCHITECTURE.md` (§3), `REGISTRO.md`.
- Estado: hecho. Pendiente para el empaquetado: adjuntar `THIRD_PARTY_NOTICES.md` al ejecutable distribuido.