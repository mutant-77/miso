# Plan de Implementación: EPUB con imágenes

## 0. Pregunta al titular

| # | Pregunta | Recomendación | Si dice que no |
|---|---|---|---|
| Q1 | ¿El título de la ventana de un EPUB es el título del libro (`dc:title`) o el nombre del archivo, como en el resto de formatos y en Tofu? | **Título del libro** (C4): es lo que el lector reconoce; el archivo de un EPUB suele llevar un nombre técnico. | Se quitan C4 y el cambio de `save_copy` de P4: `Doc.title` sigue siendo el nombre del archivo. Nada más cambia. |

El plan está escrito con la recomendación. Se responde antes de P3.

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> «EPUB con imágenes» es decisión del titular (`DIFERIDOS.md`, 2026-11-13). C1–C15 las fija el orquestador por ser las de menor código; el titular puede reabrirlas antes de implementar. C7 **toca la CSP** (§11) con el mínimo: una directiva `img-src`.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| C1 | **Detección:** un zip (`PK\x03\x04`) que abre con `ZipArchive` y contiene `META-INF/container.xml` → EPUB; cualquier otro zip sigue el camino DOCX actual (sin `word/document.xml` → `Unsupported`; zip inválido → `Corrupt`). No se exige el archivo `mimetype`. `epub::is_epub(bytes)` abre el directorio central una vez y `docx::parse` lo vuelve a abrir. | `container.xml` es obligatorio en OCF y es lo que se lee después; hay EPUB reales con `mimetype` comprimido o desordenado. Abrir el directorio central dos veces es despreciable y evita cambiar la firma de `docx::parse` y sus 15 tests. | P3 |
| C2 | **Lectura:** `container.xml` → primer `rootfile@full-path` → OPF. Del OPF: `manifest/item` (`id`, `href`, `media-type`), `spine/itemref@idref` en orden y el primer `title` de `metadata`. Se recorre el **spine entero en orden** (también `linear="no"`); sólo se leen los ítems `application/xhtml+xml` o `text/html`; un `idref` sin ítem, un `href` que no está en el zip o un capítulo que `plain::decode` rechaza se salta (`log::warn!`). NCX y `nav` no se interpretan (el documento `nav`, si está en el spine, se lee como un capítulo más). Elementos por `local_name()` de `quick-xml` (con o sin prefijo `opf:`/`dc:`). | El spine es el orden de lectura; el índice sólo sirve para navegar (fuera de alcance). Tolerar referencias rotas lee más libros reales. | P3 |
| C3 | **Capítulos con el importador HTML, sin duplicarlo:** bytes → `plain::decode` (como el HTML suelto) → se descarta todo lo anterior al primer `<body` (sin distinguir mayúsculas; si no aparece, se usa el texto entero) → `html::parse_with(texto, resolver)`. Los bloques de los capítulos se concatenan sin separador extra. | El XHTML se parsea con el parser HTML5 de `scraper`: un `<title/>` autocerrado en `<head>` abre un elemento RCDATA que se traga el capítulo. Cortar en `<body` lo evita con una línea. | P2, P3 |
| C4 | **Título:** `dc:title` pasado por `clean` y con blancos colapsados; si falta o queda vacío, el nombre del archivo. `save_copy` propone el nombre del archivo de origen (`file_stem` de `DocKey.path`) y, sin clave (portapapeles), el título. *(Pendiente de Q1.)* | Hoy `save_copy` hace `file_stem(title)`: con «Dr. No» propondría «Dr». La ruta ya vive en Rust (`DocKey`), así que no hay dato nuevo. | P3, P4 |
| C5 | **Modelo:** `Block::Image { id: u32 }` (con `ts-rs`: `{ "type": "Image", id: number }`). `Doc.images: Vec<Image>` sólo en Rust (`#[serde(skip)] #[ts(skip)]`) con `Image { id: u32, mime: &'static str, bytes: Arc<[u8]> }`. Los `id` son únicos en el proceso (`static AtomicU32` en `epub.rs`); dentro de un libro, la misma ruta da el mismo `id`. `Block::runs()` devuelve `&[]` para `Image`. | El frontend sólo recibe un número (D10). `Arc` hace barato el clon del `Doc` que ya hacen `take` y `Slots::doc`. Ids únicos en el proceso: la caché de imágenes de WebView2 es compartida entre ventanas y la URL nunca se repite con otro contenido. | P2 |
| C6 | **Índice de carácter:** una imagen es un bloque de **0 unidades UTF-16**; el separador `\n` cuenta 1, como entre cualquier bloque. `flatText` aporta `""`; la búsqueda no cambia (una coincidencia nunca cruza bloques). En el DOM es `<p data-char-start class="img"><img></p>`: sigue la correspondencia «k-ésimo `p[data-char-start]` = bloque k» de `anchor.ts`, `highlight.ts` y `measurePitch`; `firstVisibleCharIndex` sobre la imagen da el inicio del bloque (el caret cae en el `<p>`) y `charRect` usa el rectángulo del `<p>` (ya lo hace sin nodos de texto). | Es la regla de un bloque vacío, que ya existe (`p.blank`): sin casos nuevos en el ancla. Contar 1 (U+FFFC) contradiría §5 (se elimina U+FFFC) y obligaría a añadir texto al DOM. | P2, P5 |
| C7 | **Entrega al frontend por protocolo propio `miso-img`**, no `data:`. `Builder::register_uri_scheme_protocol("miso-img", …)` en `lib.rs`; el manejador toma `ctx.webview_label()` y el `id` del path, busca la imagen **sólo en el `Doc` de esa ventana** (`Slots::image`) y responde 200 con `Content-Type` = `mime`, o 404. Corre en el hilo de WebView2: `slots` se bloquea con `unwrap_or_else(PoisonError::into_inner)` (como `focused` en `lib.rs`). El frontend compone la URL con `convertFileSrc(String(id), 'miso-img')` (`http://miso-img.localhost/<id>` en Windows). **CSP:** `default-src 'self'; img-src 'self' miso-img: http://miso-img.localhost; connect-src ipc: http://ipc.localhost`. | Con `data:` los bytes viajan en el JSON del IPC en base64 (+33 %), se copian en el estado de Svelte y en cada `src`, y Rust ya clona el `Doc` dos veces: un libro ilustrado de 100 MB serían varios cientos en memoria. Con el protocolo los bytes viven una vez en Rust y el navegador los pide. Ni rutas ni red (D6, D10): sólo un número que otra ventana no puede resolver. Comprobado en `tauri` 2.12.5: Tauri no añade protocolos propios a la CSP (sólo nonces/hashes de `script-src`/`style-src`), hay que declararlos; `UriSchemeContext::webview_label()` existe. | P4 |
| C8 | **Formatos:** `image/jpeg`, `image/png`, `image/gif`, `image/webp`, `image/svg+xml`, por el `media-type` del manifiesto o, si la ruta no está en él, por extensión (`jpg`/`jpeg`/`png`/`gif`/`webp`/`svg`); cualquier otro se omite. SVG se admite **sólo como `<img>`**: en ese contexto Chromium no ejecuta scripts ni carga recursos externos. Un `<svg>` en línea con un `<image href|xlink:href>` (portadas de Calibre) produce una `Image` con ese `href`; el resto del SVG en línea se sigue ignorando. | Son los formatos de imagen principales de EPUB 3. El `Content-Type` correcto es obligatorio para SVG (Chromium no lo detecta por contenido). `scraper` guarda `xlink:href` con espacio de nombres: se busca por nombre local en `attrs()` (comprobado en `scraper` 0.29: `attr()` exige el espacio vacío). | P2, P3 |
| C9 | **Imágenes rotas o enormes:** se omiten (sin bloque) si el `src` tiene esquema (`http:`, `data:`, `file:`…), sale de la raíz del zip con `..`, no está en el zip, está cifrada, supera `LIMITE` o su tipo no es de C8. Ninguna de esas causas es un error del documento. Si el navegador no la decodifica, el `<img>` queda `hidden` (C10). | Un libro con una imagen mala se sigue leyendo. Sin red (D6). | P3, P5 |
| C10 | **Maquetación:** `p.img { break-inside: avoid }` e `img { display: block; margin-inline: auto; max-width: 100%; max-height: var(--col-h) }` (mantiene la proporción). `fit()` escribe `--col-h` = `viewport.clientHeight − 2 × vertical_inset` con una función propia que el `ResizeObserver` del viewport llama también, sola, antes de `restore()` (la barra de scroll cambia esa altura; los anchos no se tocan ahí). Tras montar, si hay imágenes, `Promise.allSettled` sobre `img.decode()` (la que falla queda `hidden`) y después, si el componente sigue montado (`scroller`), **un** `fit()` + `restore()`. | Una imagen nunca se parte y cabe en una columna. Sin dimensiones en el modelo (habría que leer cabeceras de cinco formatos), la imagen mide 0 hasta cargarse: un único `restore()` tras la carga devuelve el ancla, que no se mueve porque es un índice (D3). Un `restore()` por evento `load` maquetaría el libro entero N veces. | P5 |
| C11 | **Exportación:** texto y RTF **omiten** los bloques `Image` (ni línea vacía ni `\pict`). | `save_copy` es una copia de texto; el RTF con imágenes no se pide. | P2 |
| C12 | **HTML suelto y DOCX siguen sin imágenes.** `html::parse(texto)` = `parse_with(texto, &mut \|_\| None)`; DOCX sigue ignorando `w:drawing`/`w:pict`. Pasa a `DIFERIDOS.md`. | HTML suelto obligaría a leer archivos vecinos al elegido (más E/S de rutas) o a la red; DOCX exige leer `document.xml.rels` y `a:blip`. Lo mínimo coherente es lo que el titular pidió: EPUB. | P2, P7 |
| C13 | **Errores (sin variantes nuevas):** capítulo listado en `META-INF/encryption.xml` (`CipherReference@URI`, resuelto con `resolve("", URI)`: viene codificado) → `Protected`; si sólo hay fuentes ofuscadas, el libro abre. `Protected` pasa a decir «No se pudo abrir «{file}»: está protegido con contraseña o DRM.». `container.xml` sin `rootfile`, OPF ausente, XML mal formado (contenedor, OPF o `encryption.xml`) o spine vacío → `Corrupt` con detalle al log. Zip inválido → `Corrupt` (camino DOCX actual). Spine con capítulos que no dan bloques → documento vacío, como un archivo de 0 bytes. | Las variantes existentes encajan; sólo el texto de `Protected` se amplía para no mentir con un DRM. `encryption.xml` también existe en libros libres con fuentes ofuscadas: mirar sólo su presencia rechazaría libros legítimos. | P3 |
| C14 | **Límites y pánicos:** `LIMITE` (256 MB por parte) y la lectura de partes se mueven de `docx.rs` a `import/mod.rs` como `entry(zip, nombre) -> Result<Option<Vec<u8>>, ParseError>`, junto con `corrupt`, `attr` y la resolución de entidades XML (`reference` de `docx.rs`); `docx::part` queda como `entry` + UTF-8. EPUB y DOCX los comparten; en EPUB, un `Err` de `entry` al leer una **imagen** (p. ej. supera `LIMITE`) la omite (C9) en vez de propagarse. Sin tope total ni `catch_unwind` nuevo: el de `from_bytes` ya cubre `epub::parse`. | Reutiliza la protección existente sin duplicarla. Un tope total arbitrario rechazaría cómics legítimos; es un lector local. | P3 |
| C15 | **`parse` devuelve `Parsed { blocks, title: Option<String>, images: Vec<Image> }`** (`impl From<Vec<Block>>` para los demás importadores); `from_bytes` construye el `Doc`. Ctrl+O añade `epub` al filtro «Documentos». | Es el cambio mínimo para que EPUB aporte título e imágenes sin tocar los otros cuatro importadores. | P3 |

## 2. Objetivo

Un EPUB sin DRM abre en columnas con sus capítulos en el orden del spine, el título del libro en la ventana y sus imágenes dentro de las columnas, ajustadas a la columna y sin partirse; posición, búsqueda y copia funcionan igual que en los demás formatos.

## 3. Resumen del análisis

- **Estado heredado (`23e3469`):**
  - `import/mod.rs::parse` despacha `PK\x03\x04` → `docx::parse`; `from_bytes` envuelve en `catch_unwind` y pone `title = file`. `docx.rs` tiene `LIMITE`, `part`, `attr`, `corrupt` y la resolución de `GeneralRef` (quick-xml 0.42 emite las entidades como eventos aparte).
  - `html.rs::parse(&str) -> Vec<Block>`: `img` cae en `_ => self.children(el)` (se omite); `svg` está en `IGNORADOS`.
  - `model.rs`: cuatro variantes con `runs`. `match` exhaustivos sobre `Block`: `model.rs::runs`, `export.rs::bloque` y los `etiqueta` de los tests de `html.rs`, `docx.rs` y `rtf.rs`. Constructores literales de `Doc`: `import/mod.rs`, `clipboard.rs`, tests de `model.rs`, `doc_windows.rs` y `export.rs`.
  - `doc_windows.rs::Slots` guarda `(Doc, Option<DocKey>)` por *label* desde el pendiente hasta `close` (C2 de la fase 7): sirve para `Slots::image`.
  - Frontend: `block.runs` se usa en `textIndex.ts` (`blockLength`, `flatText`), `hyphenate.ts` (`detectLang`) y `Columns.svelte` (`attrs`, pintado). El pintado va en una sola línea para no crear nodos de texto entre bloques (§5).
  - `tauri.conf.json`: CSP `default-src 'self'; connect-src ipc: http://ipc.localhost`. En `tauri dev` no se aplica (§11).
- **APIs comprobadas en el código de las crates** (`Cargo.lock`): `tauri` 2.12.5 (`Builder::register_uri_scheme_protocol`, `UriSchemeContext::{app_handle, webview_label}`, `tauri::http` reexportado; la CSP no se amplía sola), `@tauri-apps/api` 2.12.4 (`convertFileSrc(path, protocol)` delega en `__TAURI_INTERNALS__`), `zip` 9.5.1 (`ZipArchive::index_for_name`, `by_name`), `quick-xml` 0.42.0 (`BytesStart::local_name`, `Event::GeneralRef`, `BytesRef::{is_char_ref, resolve_char_ref}`), `scraper` 0.29.0 (`Element::name` = nombre local; `attr()` sólo sin espacio de nombres; `attrs()` da nombres locales), `ts-rs` 12.0.1 (`#[ts(skip)]` en campos), `percent-encoding` 2.3.2 (ya en el grafo; `percent_decode_str(..).decode_utf8()`).
- **Supuesto, no verificado:** que `Promise.allSettled(img.decode())` llegue antes que cualquier interacción útil del usuario en libros con cientos de imágenes; que WebView2 mantenga en memoria decodificada sólo lo visible. Se mide en P6.
- **Cobertura de partida:** la del cierre de fase 7 (Rust 168, unit 102, layout 32); P1 anota la real.
- **Riesgos:**
  1. XHTML por el parser HTML5: etiquetas no vacías autocerradas en el `<body>` (`<a/>`, `<script/>`, `<textarea/>`) anidan o se tragan texto. C3 cubre el caso común (`<head>`); el resto es raro y se acepta.
  2. La CSP sólo se comprueba con un ejecutable compilado (`tauri build --debug`), que hace el titular (§4).
  3. Libros muy ilustrados: todas las imágenes se piden al abrir (la maqueta multicolumna es del libro entero, D1). Rendimiento de documentos grandes = fase 8.
  4. El manejador del protocolo es síncrono (hilo de la interfaz en WebView2) y copia los bytes de cada imagen una vez; con imágenes de pocos MB es inapreciable.

## 4. Restricciones innegociables

- El orquestador nunca ejecuta `pnpm build` ni `pnpm tauri build`; `pnpm tauri dev` sí. Puertas desde PowerShell. Puerta común: `pnpm verify`.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D3/D9 (índice UTF-16; el DOM no añade texto), D6 (sin red), D10 (el frontend no envía ni recibe rutas), D11 (el frontend no muestra errores).
- Los tipos de `src/lib/ipc/types/` sólo los genera `cargo test`.

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos (`model.rs`, `import/{mod,html,docx,plain}.rs`, `commands.rs`, `doc_windows.rs`, `error.rs`, `export.rs`, `lib.rs`, `Cargo.toml`, `tauri.conf.json`, `Columns.svelte`, `Reader.svelte`, `textIndex.ts`, `anchor.ts`, `columnScroll.ts`, `highlight.ts`, `search.ts`, `hyphenate.ts`, `commands.ts`, `tests/layout/*`)
- [x] APIs de las crates comprobadas en `~/.cargo/registry/src` (§3)
- [ ] Respuesta del titular a Q1
- [ ] Dos o tres EPUB reales del titular para P6 (una novela con portada, uno ilustrado; uno con DRM si lo tiene)
- [x] Foto temporal de prueba en `tests/fixtures/Krystle Cole mask 1997.jpg` (8,7 MB): sólo para P6, nunca se sube; se borra en P7 junto con cualquier test o EPUB de prueba que la use.

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- `git status` limpio salvo este plan; `pnpm verify` en verde; anotar recuentos. Punto de vuelta atrás: `23e3469`.

**Gate:** `pnpm verify` = 0 sobre `23e3469`.

### Paso 2 — Modelo e importador HTML con imágenes
**Archivos:** `src-tauri/src/{model,export,clipboard}.rs`, `src-tauri/src/import/{mod,html,docx,rtf}.rs` (sólo constructores y `etiqueta` de tests), `src-tauri/src/doc_windows.rs` (constructor de test), `src/lib/ipc/types/*` (regenerados)
- `model.rs`: C5. `Image` y `Doc.images`; `runs()` con `Image`. Los constructores literales de `Doc` añaden `images: Vec::new()`.
- `export.rs`: C11 (filtrar `Image` en `plain` y `rtf`).
- `html.rs`: `parse_with(text, image: &mut dyn FnMut(&str) -> Option<u32>)`; `parse` delega con un resolutor que devuelve `None` (C12). `img[src]` y `svg` con `image[href|xlink:href]` (C8): si el resolutor da `id` → `flush()` + `Block::Image { id }`; con `None` no se toca el bloque abierto (el HTML suelto no parte párrafos). Con imagen, el bloque abierto sigue con su tipo (un `ListItem` sigue sin marcador repetido). Dentro de una fila de tabla, nada.
- Tests Rust: `flat_len` con `[P "ab", Image, P "c"]` = 5; `parse_with` emite `[p "a", img, p "b"]` para `<p>a<img src="x">b</p>`; `<svg><image xlink:href="c.jpg"/></svg>` y `href` → `Image`; `parse` (sin resolutor) no emite imágenes y deja `<p>a<img src="x">b</p>` como un párrafo «ab»; `<img>` en `<td>` no parte la fila; export omite `Image` (texto y RTF). Los `etiqueta` de tests ganan `Image`.
- **Negativa:** quitar el `flush()` previo a la imagen hace fallar el test del párrafo partido.

**Gate:** `pnpm lint:rust && pnpm test:rust` = 0 y `Block.ts` regenerado con `{ "type": "Image", id: number }`. `pnpm verify` completo puede fallar en `check` hasta P5 (el frontend aún lee `block.runs`): se anota y no se hace commit intermedio.

### Paso 3 — Importador EPUB
**Archivos:** `src-tauri/src/import/{mod,docx}.rs`, `src-tauri/src/import/epub.rs` (nuevo), `src-tauri/src/error.rs`, `src-tauri/src/commands.rs` (filtro), `src-tauri/Cargo.toml` (`percent-encoding = "2.3"`)
- `mod.rs`: C14 (mover helpers), C15 (`Parsed`), C1 (despacho), C4 (título en `from_bytes`).
- `epub.rs` (~200 líneas): `is_epub`; `parse` según C2, C3, C8, C9, C13; `resolve(base_dir, href) -> Option<String>` (quita `#…`/`?…`, rechaza esquemas, decodifica `%XX`, normaliza `.`/`..`, nunca por encima de la raíz; sólo cadenas con `/`, sin `Path`; con `base_dir` vacío no deja `/` inicial); resolutor de imágenes por capítulo (aporta su `base_dir`) sobre **una** caché ruta → `id` por libro.
- `error.rs`: texto de `Protected` (C13) y su fila en `serializa_a_kind_y_message`.
- Tests Rust con `ZipWriter` en memoria (`mimetype` *stored* primero, `META-INF/container.xml`, `OEBPS/content.opf`, `OEBPS/Text/*.xhtml`, `OEBPS/Images/*`; bytes de imagen arbitrarios, Rust no decodifica):
  - orden del **spine** distinto del manifiesto; `linear="no"` se lee; ítem no XHTML y `idref` roto se saltan; OPF en la raíz del zip (`full-path="content.opf"`) lee sus capítulos; capítulo con `<BODY>` en mayúsculas;  - título con `&amp;` y blancos → limpio; sin `dc:title` → nombre del archivo (por `from_bytes`);
  - `<title/>` en `<head>` no se traga el capítulo;
  - `../Images/a%20b.png` resuelve; la misma imagen dos veces → mismo `id` y una sola entrada en `images`; `mime` del manifiesto; SVG por extensión sin entrada de manifiesto; `http://`, `data:`, `../../x.png`, inexistente y `image/tiff` → omitidas; ids distintos entre dos importaciones;
  - EPUB sin `mimetype` se detecta; zip con `word/document.xml` sigue siendo DOCX; zip sin ninguno → `Unsupported`;
  - capítulo cifrado → `Protected`; `encryption.xml` sólo con una fuente → abre; sin `rootfile`, OPF inexistente, OPF mal formado, spine vacío → `Corrupt`; EPUB truncado → `Corrupt` (vía `from_bytes`).
  - Los tests de `docx.rs` siguen verdes tras mover los helpers.
- **Negativas:** recorrer el manifiesto en vez del spine hace fallar el test de orden; ignorar `encryption.xml` hace fallar el de `Protected`; mirar sólo la presencia de `encryption.xml` hace fallar el de la fuente.

**Gate:** `pnpm lint:rust && pnpm test:rust` = 0 (`check` como en P2).

### Paso 4 — Servir las imágenes
**Archivos:** `src-tauri/src/{doc_windows,lib,commands}.rs`, `src-tauri/tauri.conf.json`
- `doc_windows.rs`: `Slots::image(label, id) -> Option<(&'static str, Arc<[u8]>)>` (pendiente u abierto) y `image_response(slots, label, path) -> http::Response<Vec<u8>>` pura (C7).
- `lib.rs`: `register_uri_scheme_protocol("miso-img", |ctx, req| …)` que bloquea `slots` y delega en `image_response`.
- `commands.rs`: nombre propuesto de `save_copy` según C4.
- `tauri.conf.json`: CSP de C7.
- Tests Rust: 200 con `Content-Type` y los bytes; 404 para `id` desconocido, path no numérico, **otra ventana** y ventana cerrada.
- **Negativa:** buscar el `id` en todas las ventanas en vez de en la de `webview_label()` hace fallar el test de otra ventana.

**Gate:** `pnpm lint:rust && pnpm test:rust` = 0.

### Paso 5 — Frontend
**Archivos:** `src/lib/layout/textIndex.ts` (+ test), `src/lib/layout/hyphenate.ts`, `src/lib/ipc/commands.ts`, `src/components/Columns.svelte`, `tests/layout/columns.test.ts` (sus `block.runs` pasan a `runsOf`), `tests/layout/images.test.ts` (nuevo)
- `textIndex.ts`: `runsOf(block)` (`[]` para `Image`), usado en `blockLength`, `flatText` y `detectLang`.
- `commands.ts`: `imageSrc(id) = convertFileSrc(String(id), 'miso-img')`.
- `Columns.svelte`: rama `Image` primero, en la misma línea, `<p data-char-start={starts[i]} class="img"><img src={imageSrc(block.id)} alt="" /></p>`; CSS y espera de C10; `--col-h` en `fit()`.
- Test unit: `[P "ab", Image, P "c"]` → `flatText` `"ab\n\nc"`, `flatLength` 5 (= Rust), `blockStarts` `[0, 3, 4]`.
- Tests de layout (stub `window.__TAURI_INTERNALS__ = { convertFileSrc }` que devuelve PNG `data:` de un `canvas`; en el navegador de test no hay CSP):
  - imagen de 200×3000 → su rectángulo cabe en la altura de columna y en una sola columna; imagen de 3000×100 → ancho ≤ columna;
  - `initial_char_index` en mitad de un libro con imágenes antes del ancla → tras cargar, el carácter está en la primera columna visible (como «monta en el carácter inicial»);
  - con una imagen al inicio de una columna, `firstVisibleCharIndex` = inicio del bloque imagen y `columnOfCharIndex` de ese índice = esa columna;
  - `measurePitch` correcto con una imagen como primer bloque; imagen rota → `hidden` y el ancla se conserva.
- **Negativas:** quitar el `restore()` posterior a `decode()` hace fallar el test de posición inicial (si las imágenes `data:` cargan tan rápido que no falla, se agranda o multiplica la imagen del test hasta que muerda); quitar `max-height` hace fallar el de la imagen alta; contar la imagen como 1 hace fallar el unit.

**Gate:** `pnpm verify` = 0.

### Paso 6 — Comprobación
**Orquestador** (`pnpm tauri dev`; lo visual es del titular):
- Los EPUB del titular abren por Ctrl+O (filtro «Documentos») y por arrastre; título de ventana = libro; imágenes servidas (log sin errores); reabrir vuelve a la posición; buscar una palabra de un capítulo tardío la encuentra; Ctrl+S `.txt` y `.rtf` reabren sin imágenes. Un EPUB con DRM (o uno generado con un capítulo cifrado) muestra el aviso `Protected`.

**Titular:**
- Imágenes centradas, sin partir, ajustadas a la columna; cambiar fuente, ancho y escala conserva el punto de lectura; pantalla completa igual.
- CSP: `pnpm tauri build --debug`, abrir `src-tauri/target/debug/miso.exe`, F12: un EPUB con imágenes se ve completo y la consola no muestra violaciones; `fetch('https://example.com')` sigue bloqueado.

**Gate:** confirmación del titular.

### Paso 7 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `DECISIONES.md`, `REGISTRO.md`, este plan
- `ARCHITECTURE.md`: §4 (`import/epub.rs`; `mod.rs` con helpers de zip/XML; `Block.svelte` no existe, el pintado vive en `Columns.svelte`), §5 (modelo con `Image`, `Doc.images` sólo en Rust, imagen = 0 unidades; «Contenido no representado»: imágenes sólo en EPUB; protocolo `miso-img`; filas `Corrupt` y `Protected` con EPUB y DRM), §6 (`p.img`, `--col-h`, `restore()` tras `decode()`), §9.5 (zip con `container.xml` → EPUB) y fila EPUB en la tabla de importadores, §11 (CSP con `img-src`; el protocolo sólo sirve las imágenes de la ventana que pide, por número), §12 (EPUB en memoria; layout con imágenes), §13 (fila «EPUB ✅»).
- `DIFERIDOS.md`: fuera la entrada EPUB; nueva (sin fase): imágenes en DOCX (`document.xml.rels` + `a:blip`) y HTML suelto.
- `DECISIONES.md`: «EPUB con imágenes por protocolo propio» y la respuesta a Q1.
- `REGISTRO.md`: asiento. Este plan → `.historial/_PLAN_EPUB_completado.md`.
- Borrar `tests/fixtures/Krystle Cole mask 1997.jpg` y el test o EPUB de prueba que la use (§5); no entran en el commit.
- Commit `feat: EPUB con imágenes`.

**Gate:** `pnpm verify` = 0 sobre el commit final; `git status` limpio.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─┐
       └──────→ P5 ───┴─→ P6 ─→ P7
```
P4 y P5 no comparten archivos; P5 necesita el `Block.ts` de P2 y P6 necesita P3–P5.

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 sobre `23e3469` | Rust 168, unit 102, layout 32. Q1: título del libro. |
| P2 — Modelo e HTML | ✅ completado | `lint:rust` + `test:rust` = 0 (176); negativa del `flush()` cumplida | `#[allow(dead_code)]` temporal en `Image`/`Doc.images` hasta P4. |
| P3 — Importador EPUB | ✅ completado | `pnpm verify` = 0 (Rust 200, unit 103, layout 38 ×3 estable); negativas (a)(b)(c) cumplidas | Desvíos en §9. |
| P4 — Servir imágenes | ✅ completado | `pnpm verify` = 0 (Rust 209, unit 103, layout 38); negativa «otra ventana» cumplida | `#[allow(dead_code)]` de P2 retirado. `nombre_propuesto` pura con 3 tests. |
| P5 — Frontend | ✅ completado | `check` 0, unit 103, layout 38; negativas (a)(b)(c) cumplidas | En paralelo con P3. Desvíos en §9. |
| P6 — Comprobación | ✅ completado (CSP en ejecutable → DIFERIDOS) | Humo con el importador real: EPUB de prueba → título «Dr. No & la máscara», 3 bloques imagen (2 imágenes, JPG 8,7 MB + SVG), externa e inexistente omitidas, palabra tardía presente; EPUB con capítulo cifrado → `Protected` | Falta la comprobación en la app (titular). |
| P7 — Cierre y commit | ✅ completado | `pnpm verify` = 0 antes del commit | Titular aprueba «composición de libro» (plan aparte). |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- CSS del EPUB, fuentes embebidas, tamaños y colores del libro (como el resto de formatos, diferido general).
- Índice navegable (NCX/`nav`), enlaces internos, notas al pie navegables, portada especial, capítulo en columna nueva.
- Texto alternativo (`alt`) de las imágenes, leyendas fuera de su `figcaption` de texto, imágenes en línea dentro de un párrafo (toda imagen es un bloque).
- Dimensiones de imagen en el modelo, carga diferida, tope total de memoria por libro.
- EPUB de maquetación fija (*pre-paginated*) más allá de leer sus imágenes en orden; audio, vídeo, MathML y scripts.
- DRM: no se descifra nada; sólo se detecta.
- Imágenes en DOCX y HTML suelto (C12, a `DIFERIDOS.md`); imágenes en la copia exportada (C11).

**Hallazgos aparcados**
- Manifiestos con `media-type` no estándar (`image/jpg`) omiten la imagen por C8 → `DIFERIDOS.md`.
- P6 con *La acción humana* y *Libro del desasosiego*: sin la hoja de estilos del libro, los párrafos no se distinguen y un capítulo empieza a media columna. El titular decide una composición de libro propia de Miso → `DECISIONES.md`, plan aparte.

**Desvíos de la implementación**
- P3: `resolve` trata un `href` que empieza por `/` como relativo a la raíz del zip. El test de «sale de la raíz» usa `../../../x.png`: desde `OEBPS/Text`, `../../x.png` es `x.png`, que está dentro de la raíz (se omite por inexistente).
- P3: un capítulo cuyo `entry` falla (supera `LIMITE`) propaga `Corrupt`; sólo las imágenes se omiten (C9/C14).
- P5: `Promise.all` con `.catch` por imagen en vez de `Promise.allSettled` (mismo efecto; el `lib` es2017 de `tsconfig` no tipa `allSettled`).
- P5: `display: block` en `img:not([hidden])`: con `img { display: block }` el atributo `hidden` de una imagen rota no ocultaba nada.
- P5: los tests de imágenes usan URLs `blob:` (carga asíncrona como `miso-img`); con `data:` la negativa del `restore()` no mordía.
