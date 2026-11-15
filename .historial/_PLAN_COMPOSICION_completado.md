# Plan de Implementación: Composición de libro

## 0. Preguntas al titular

| # | Pregunta | Recomendación | Si dice otra cosa |
|---|---|---|---|
| Q1 | ¿«Capítulo» existe sólo en EPUB, o también son capítulo los saltos de página de DOCX (`w:br w:type="page"`, `w:pageBreakBefore`) y RTF (`\page`, `\sect`)? | **Sólo EPUB** (C4). Es el caso comprobado (*La acción humana*); en Word el salto de página se usa a menudo para maquetar (empujar una tabla, una portada), y leerlo exige tocar dos importadores más. Pasa a `DIFERIDOS.md`. | Se añade a P3 la lectura de esos saltos en `docx.rs` y `rtf.rs` (hoy `\page`/`\sect` sólo cierran párrafo, `rtf.rs:215`; `docx.rs` ignora el tipo de `w:br`), con sus tests. El frontend no cambia. |
| Q2 | En EPUB, cada archivo del spine abre columna: en *Libro del desasosiego* (Calibre, 518 archivos `index_split_NNN`) **cada fragmento** empieza en columna nueva, y en los libros con notas en archivo aparte, cada bloque de notas también. ¿Se acepta? | **Sí.** Calibre parte por los saltos de página del original (`id="calibre_pb_N"`): cada fragmento ya iba en página nueva, y Sumatra hace lo mismo. Distinguir «parte de tamaño» de «capítulo» exigiría heurísticas por nombre de archivo. | Habría que decidir una heurística (p. ej. no abrir columna si el archivo anterior no acaba en fin de frase); se replanifica C4. |
| Q3 | En composición de libro, una línea en blanco (bloque vacío o sólo de espacios, p. ej. los `<p>&nbsp;</p>` de Calibre o los párrafos vacíos de Word) **no se ve** (alto 0): el blanco entre párrafos ya lo pone la composición. ¿De acuerdo? | **Sí** (C7). Esos blancos imitaban la separación que ahora pone Miso; sumarlos daría huecos de línea y media. Se pierde el «doble blanco» de cambio de escena en DOCX; es lo menos frecuente. | Los blancos mantienen su media línea (`p.blank` actual) y se suman a la separación: sólo cambia una regla CSS y el test (b) de P4. |

El plan está escrito con las recomendaciones. Se responden antes de P3.
**Respondidas (2026-11-14):** Q1, Q2 y Q3 = recomendación.

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> «Composición de libro» es decisión del titular (`DECISIONES.md`, 2026-11-14), con su memoria: los defectos de presentación se corrigen, no son una opción de Preferencias. C1–C12 las fija el planificador por ser las de menor código; el titular puede reabrirlas antes de implementar.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| C1 | **Modelo:** `Doc` gana dos campos, exportados por `ts-rs`, en este orden: `title, initial_char_index, composition, chapters, blocks`. `composition: Composition` con `#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)] enum Composition { Text, Book }` (TS: `"Text" \| "Book"`). `chapters: Vec<u32>` = índices de **bloque** donde empieza un capítulo, crecientes, nunca `0`. Ambos obligatorios (sin `skip_serializing_if`). | El frontend no puede deducir el formato del título (portapapeles, EPUB con `dc:title`). Índices de bloque y no un bloque separador: un bloque nuevo sumaría un `\n` al texto aplanado y movería todas las posiciones guardadas (D3/D9). | P3 |
| C2 | **Quién es `Book`:** `import::parse` decide; `Parsed` gana `composition` y `chapters`; `impl From<Vec<Block>> for Parsed` pone `Book` y `chapters` vacío (PDF, RTF, DOCX, HTML); la rama de texto plano construye `Parsed { composition: Text, .. }`. `clipboard.rs` crea `Text`. `from_bytes` copia ambos campos al `Doc`. | Cinco de seis caminos son `Book`: el `From` existente lo da con una línea. El portapapeles es texto plano, con la misma excepción que el `.txt` (versos, listas). | P3 |
| C3 | **Índice intacto:** la composición es sólo CSS sobre los `<p>` existentes: `text-indent`, `margin-block-start`, `text-align`, `break-before`, `height`. Ni `::before`/`::after` con contenido, ni nodos nuevos, ni texto. `flat_len`, `blockStarts`, `flatText`, búsqueda, copia y exportación no cambian. | D3/D9 (§5: «El DOM no puede añadir texto»). | P4 |
| C4 | **Capítulo = ítem del spine EPUB que aporta al menos un bloque.** En `epub.rs::parse`, por cada capítulo leído: `nuevos = html::parse_with(..)`; si está vacío, no cuenta; si no y `blocks` ya tiene bloques, `chapters.push(blocks.len() as u32)`; después `blocks.extend(nuevos)`. Sin capítulos en el resto de formatos *(pendiente de Q1)*. Las notas en archivo aparte y los fragmentos de Calibre abren columna *(Q2)*. | Es la frontera que ya existe en el código (`epub.rs:63-90` concatena los capítulos). Un ítem vacío (portada sin imagen legible, `nav` vacío) no debe dejar una columna en blanco. | P3 |
| C5 | **Clases por bloque calculadas en TS, no en CSS de hermanos:** `src/lib/layout/composition.ts` exporta `bookClasses(doc: Doc): string[]` (una cadena por bloque, clases separadas por espacio). Para `Text` devuelve `''` en todos. Para `Book`, recorriendo los bloques con `prev` = último bloque **no blanco** del mismo capítulo (se reinicia a `null` en cada índice de `chapters`): **`blank`** si es blanco (C7) y nada más; **`chapter`** en el primer bloque no blanco de cada capítulo de `chapters` (el capítulo de un bloque es el último índice de `chapters` ≤ su posición; `chapter` va al primer no blanco **de ese rango** y, si no hay ninguno, nadie la lleva); **`sep`** si `prev !== null` (por tanto nunca en el bloque `chapter`: `chapter` sustituye a `sep`) y no son `ListItem` seguido de `ListItem`; **`indent`** si es `Paragraph`, `prev !== null` y `prev` no es `Heading` ni `Image`. `attrs()` de `Columns.svelte` une esas clases con las suyas (`kind`, `blank`) sin duplicados. | Un selector `h + p` no ve a través de los blancos de Calibre (`<h2>…</h2><p>&nbsp;</p><p>texto`), y el inicio de capítulo no es un hermano. Una función pura se prueba en `node` sin maquetar. | P4 |
| C6 | **CSS** (en `Columns.svelte`; `.columns` lleva la clase `book` si `doc.composition === 'Book'`): títulos `text-align: center` (en ambas composiciones; sustituye `start`); `.book p.sep { margin-block-start: 0.5lh }`; `.book p.indent { text-indent: 1.5em }`; `.book p.chapter { break-before: column; margin-block-start: 0 }`; `.book p.blank { height: 0; line-height: 0 }` (sin `font-size: 0`: el carácter del blanco conserva rectángulos para `charRect`). La separación es **margen**, no padding: Chromium trunca los márgenes junto a un salto de columna **no forzado** (CSS Fragmentation §5.2), así que el blanco no deja hueco arriba; tras un salto **forzado** el margen se conserva, de ahí `margin-block-start: 0` en `chapter`. `0.5lh` es la del propio bloque: entre párrafos, media línea del cuerpo (la medida de `p.blank`); antes de un título, media línea del título (algo más, como en un libro). | Es la regla «sangrías con padding, nunca margin» de §6 en su terreno: el padding horizontal mantiene la caja de cada `<p>` en el borde de su columna (`measurePitch`); el margen vertical no toca esa caja (`getClientRects` da el borde, sin margen) y es lo único que el navegador trunca en el salto. La sangría es `text-indent`, dentro de la caja. 1.5em es la medida ya usada por listas y citas. | P2, P4 |
| C7 | **Blanco en `Book`:** bloque no imagen cuyos runs son todos sólo espacio (`/^\s*$/`, incluye U+00A0). Se queda en el DOM con su texto (el índice lo cuenta) pero mide 0 de alto y no lleva `sep`, `indent` ni `chapter`; el siguiente bloque no blanco lleva su `sep` normal. En `Text` nada cambia: `blank` sigue siendo «todos los runs `''`» y mide media línea. *(Pendiente de Q3.)* | Los `<p>&nbsp;</p>` de Calibre llegan como texto U+00A0 (comprobado en *Libro del desasosiego*, `index_split_100.xhtml`) y hoy se pintan como una línea entera. `display: none` rompería el ancla: `charRect` no tendría rectángulos y `columnOfCharIndex` devolvería 0. | P4 |
| C8 | **Ancla sin cambios de algoritmo:** `firstVisibleCharIndex` sigue sondeando en `(left + pad + 1, top + pad + línea/2)`. Con C6 la primera línea de cada columna empieza en `top + pad` (margen truncado o anulado), y el punto, dentro de la sangría o a la izquierda de un título centrado, debe dar el inicio de esa línea. P2 lo comprueba en Edge antes de construir; si no se cumple, el plan se bloquea (⛔) y vuelve al titular. Única corrección prevista: si el test de P4 «ancla sobre un blanco» falla, `charRect` (`anchor.ts`) cae a `p.getClientRects()` cuando el `Range` del carácter no da rectángulos, en vez de a `range.getBoundingClientRect()` (que vale 0,0 y devolvería la columna 0). | No se toca lo que funciona (D3, sin deriva). El riesgo está en la geometría, y se mide. | P2, P4 |
| C9 | **`measurePitch`, `columnScroll`, búsqueda y resaltado no cambian.** `text-indent` y el centrado no mueven la caja del `<p>`; un capítulo nunca deja columnas vacías entre medias (un salto forzado al inicio de columna no crea otra). | §6: el paso se mide con cajas, no con líneas. | P4 |
| C10 | **Tests de layout con las dos composiciones:** `longDoc(n, composition = 'Text')` en `tests/layout/defaults.ts` devuelve también `composition` y `chapters: []`; los literales `Doc` de los tests ganan los dos campos (los existentes, `'Text'`: su comportamiento no cambia). Los tests nuevos de libro van en `tests/layout/book.test.ts`. | `tsconfig.json` incluye `tests/`: `pnpm check` exige los campos. Los tests actuales quedan como red de `Text`. | P4 |
| C11 | **Exportación y posiciones intactas:** `export.rs` y `positions.rs` no leen los campos nuevos; `save_copy` sigue siendo texto. Abrir un libro con posición guardada vuelve al mismo carácter. | El índice no cambia (C3). | P3 |
| C12 | **Fuera de Preferencias:** no hay interruptor de composición. | Memoria del titular: los defectos de presentación se corrigen, no se ofrecen como opción. | todo |

## 2. Objetivo

Que un documento que no es texto plano se lea como un libro: títulos centrados, sangría de primera línea (salvo tras título, imagen o inicio de capítulo), media línea entre párrafos sin hueco arriba de la columna, y en EPUB cada capítulo en columna nueva. El `.txt` y el portapapeles se pintan como hoy. Posición de lectura, búsqueda y copia no cambian.

## 3. Resumen del análisis

- **Stack:** Tauri 2.12.5, Rust (serde, `ts-rs` 12 → `src/lib/ipc/types/` en `cargo test`), Svelte 5.57, TypeScript 6, Vitest 5.1 (proyecto `layout` en el Edge/EdgeCore instalado, Playwright 1.65). Puerta: `pnpm verify`.
- **Modelo (`model.rs`):** `Doc { title, initial_char_index, blocks, images (sólo Rust) }`; `Block` = `Paragraph | Heading{level} | ListItem{depth} | Quote | Image{id}`. Constructores literales de `Doc`: `import/mod.rs:133`, `clipboard.rs:8`, `doc_windows.rs:263` (test), `export.rs:87` (test), `model.rs:96,115` (tests). Fixture `tests/fixtures/unicode.doc.json` = serialización exacta de `unicode.txt` (`import::tests::unicode_coincide_con_su_json`); la importan `textIndex.test.ts` y `columns.test.ts`.
- **Importadores:** `import::parse` despacha PDF/RTF/EPUB/DOCX/HTML/texto; `Parsed { blocks, title, images }` con `From<Vec<Block>>`. `epub.rs:62-90` concatena los capítulos del spine sin marca. `html.rs::flush` descarta bloques vacíos pero conserva los de sólo U+00A0 (blancos ASCII colapsados, U+00A0 no); DOCX (`docx.rs:383`) y RTF (`rtf.rs:413`) emiten párrafos vacíos como `""` (hoy `p.blank`, media línea). PDF: todo `Paragraph`.
- **Frontend:** `Columns.svelte` pinta un `<p data-char-start>` por bloque en una sola línea (sin nodos entre bloques); `attrs()` da `class` (`h1`…`h6`, `li`, `quote`, `blank`) y `--depth`. CSS: `p { margin: 0 }`, `p.blank { height: 0.5lh }`, títulos `text-align: start; break-after: avoid`, `.li` con `padding` + `text-indent: -1.5em` (ya hay una sangría por `text-indent` y el ancla convive con ella), `.quote` con `padding-inline`.
- **Ancla (`anchor.ts`, `columnScroll.ts`):** `firstVisibleCharIndex` = `caretPositionFromPoint(vp.left + pad.left + 1, vp.top + pad.top + line/2)` → `data-char-start` + desplazamiento sin guiones blandos. `columnOfCharIndex` = `floor((rect.left − vp.left + scrollLeft − pad + 1) / pitch)` con el último rectángulo del carácter; sin nodo de texto, el último rectángulo del `<p>`. `measurePitch` = diferencia de `left` entre cajas de `<p>` de dos columnas. `restore()` mide, va a la columna del ancla y fija `anchoredLeft`. `highlight.ts::rangeOf` usa `nodeAt` sobre el k-ésimo `<p>`.
- **Libros reales (inspeccionados):** *La acción humana*: spine `cubierta, sinopsis, titulo, info, prefacio, estudio, intro, parte1, cap1…`; `intro.xhtml` empieza con `<h1>INTRODUCCIÓN</h1><h3>…</h3><p class="asangre">`. *Libro del desasosiego*: 518 archivos `index_split_NNN.xhtml`, uno por fragmento (`<h2 id="calibre_pb_N">96</h2><p>&nbsp;</p><div><p>…`), CSS del libro con `margin: 1em` en `p`. *Ideas fuertes* y *La pasión de la pregunta*: Calibre con pocas partes grandes.
- **Cobertura de partida (REGISTRO, EPUB):** Rust 209, unit 103, layout 38. Tests de ancla (`anchor.svelte.test.ts`): ancla tras 7 cambios, sin deriva ×20, primer carácter de cada columna en su columna, 500 saltos, posición inicial, búsqueda, pantalla completa. `columns.test.ts`: D9, paso, `p.blank` = media línea, documento rico. `images.test.ts`: imágenes y ancla.
- **Riesgos:**
  1. Truncado de márgenes en saltos no forzados y `break-before: column` dentro de `column-fill: auto`: comportamiento de Chromium, no del código. Se mide en P2 antes de construir.
  2. `caretPositionFromPoint` en la zona de la sangría o a la izquierda de un título centrado: debe dar el inicio de la línea; si da `null` o el final del bloque anterior, el ancla salta. P2 y P4.
  3. Bloques blancos de alto 0: `charRect` de un carácter de tamaño 0 puede no dar rectángulos (C8).
  4. Libros con cientos de capítulos cortos (Q2) dejan columnas medio vacías: más columnas, mismo rendimiento de maqueta (fase 8).
  5. Títulos que el libro marca como `<p class="titulo">` en vez de `<hN>`: no se centran (no leemos la CSS del libro). Se acepta (fuera de alcance).

## 4. Restricciones innegociables

- El orquestador nunca ejecuta `pnpm build` ni `pnpm tauri build`; `pnpm tauri dev` sí. Puertas desde PowerShell. Puerta común: `pnpm verify`.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D3/D9: el DOM no añade texto ni nodos; el índice UTF-16 no cambia. D6, D10, D11 intactas.
- Los tipos de `src/lib/ipc/types/` sólo los genera `cargo test`.
- Sin dependencias nuevas. Sin opción en Preferencias (C12).
- Ningún test existente se relaja: los de `Text` deben pasar sin tocar sus aserciones (sólo los literales `Doc`, C10).

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos: `model.rs`, `import/{mod,epub,html,docx,rtf,plain,pdf}.rs`, `clipboard.rs`, `Columns.svelte`, `anchor.ts`, `columnScroll.ts`, `textIndex.ts`, `highlight.ts`, `main.ts`, `tsconfig.json`, `vitest.config.ts`, `tests/layout/*`, `tests/fixtures/unicode.doc.json`
- [x] EPUB reales inspeccionados (spine, XHTML y CSS de *La acción humana* y *Libro del desasosiego*)
- [x] Patrones: plan EPUB (`.historial/_PLAN_EPUB_completado.md`), sonda negativa de `hyphens` (fase 2) como precedente de P2
- [x] Respuestas del titular a Q1–Q3
- [x] P2 en verde (premisas de Chromium)

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- `git status` limpio salvo este plan; `pnpm verify` en verde; anotar recuentos. Punto de vuelta atrás: `f896d60`.

**Gate:** `pnpm verify` = 0 sobre `f896d60`.

### Paso 2 — Sonda: premisas de Chromium
**Archivos:** `tests/layout/multicol.test.ts` (cuatro tests nuevos, DOM a mano como el existente; quedan como premisas permanentes, igual que D1).
Contenedor multicolumna con `column-fill: auto`, alto fijo, `padding: 20px 15px` (como `.columns`), `line-height` fijo en px, `<p>` con `margin: 0` salvo lo indicado. **Toda medida vertical (aquí y en P4) se hace sobre la caja del `<p>`** (`p.getClientRects()`, el fragmento cuyo `left` es el de la columna), nunca sobre rectángulos de `Range` de texto: la caja empieza en el borde del contenido y no depende de la fuente ni del interlineado.
- (a) **Truncado:** 200 párrafos con `margin-block-start: 0.5lh` (salvo el primero). Para cada columna k ≥ 1, el `top` de la primera caja de la columna = `top` del contenido del contenedor (±1 px).
- (b) **Salto forzado:** el párrafo 120 con `break-before: column; margin-block-start: 0`: su primer rectángulo está en una columna posterior a la del último carácter del 119 y su `top` = `top` del contenido (±1 px); con `margin-block-start: 0.5lh` (sin anular) el `top` queda 0.5 línea más abajo (esto se afirma, documenta por qué C6 lo anula).
- (c) **Sonda del ancla:** columna que empieza con un párrafo con `text-indent: 1.5em` y otra con un `<p style="text-align:center">` corto: `caretPositionFromPoint(cont.left + 15 + 1, cont.top + 20 + línea/2)` (`cont` = `getBoundingClientRect()` del contenedor, más su padding) devuelve el nodo de texto de ese `<p>` con `offset` 0.
- (d) **Blanco de alto 0:** un `<p>` con `height: 0; line-height: 0` como último bloque de una columna llena, seguido de un `<p>` con margen de (a): la primera caja de la columna siguiente sigue en el `top` del contenido (±1 px); y seguido de un bloque con `break-before: column`, el número de columnas sólo crece en 1 (sin columna vacía).
- **Negativa:** cambiar en (a) `margin-block-start` por `padding-block-start` hace fallar (a).

**Gate:** `pnpm test:layout` = 0 con los cuatro tests. Si falla (a), (c), o en (b) el bloque de capítulo no abre columna nueva o no queda en el `top` del contenido → ⛔, se para y se informa al titular (C6/C8 no valen). Si sólo falla la segunda mitad de (b) (el margen no se conserva tras un salto forzado), se anota en §8 y `margin-block-start: 0` de `.chapter` se mantiene como red; no bloquea. Si falla (d), `.book p.blank` gana `break-before: avoid`, se repite (d) y se anota en §8; si sigue fallando → ⛔.

### Paso 3 — Modelo e importadores
**Archivos:** `src-tauri/src/model.rs`, `src-tauri/src/import/{mod,epub}.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/doc_windows.rs` y `src-tauri/src/export.rs` (sólo constructores de test), `tests/fixtures/unicode.doc.json`, `src/lib/ipc/types/{Doc,Composition}.ts` (regenerados)
- `model.rs`: C1 (`Composition`, campos en `Doc`); los dos tests de `flat_len` construyen con `Composition::Book` y `chapters: Vec::new()`.
- `import/mod.rs`: C2 (`Parsed` + `From`, rama de texto plano `Text`, `from_bytes` copia los campos).
- `epub.rs`: C4; `Parsed { chapters, composition: Book, .. }`.
- `clipboard.rs`: `Text`, `chapters: Vec::new()`.
- `unicode.doc.json`: añadir `"composition": "Text", "chapters": []` tras `initial_char_index`.
- Tests Rust:
  - `epub.rs`: tres capítulos `[c1: 2 párrafos, c2: vacío (sólo <body/>), c3: 1 párrafo]` → `chapters == [2]`; un solo capítulo → `chapters == []`; capítulo que sólo da una imagen → cuenta como capítulo; `composition == Book`.
  - `import/mod.rs`: ese mismo EPUB por `from_bytes("libro.epub", …)` → `doc.composition == Book && doc.chapters == [2]` (el `Doc`, no `Parsed`: es el único camino al frontend).
  - `import/mod.rs`: `from_bytes("a.txt", …)` → `Text`; `.html`, `.rtf` y un DOCX mínimo (`ZipWriter` como en `docx.rs`) → `Book`, `chapters` vacío.
  - `clipboard.rs`: `texto_da_documento_titulado` comprueba `Text`.
  - `unicode_coincide_con_su_json` sigue verde con el JSON nuevo.
- **Negativas:** empujar el índice también para capítulos vacíos hace fallar el test `[2]`; poner `Book` en la rama de texto hace fallar `unicode_coincide_con_su_json` y el de `.txt`; quitar la copia de `chapters` en `from_bytes` hace fallar el test del `Doc`.

**Gate:** `pnpm lint:rust && pnpm test:rust` = 0; `Doc.ts` regenerado con `composition: Composition, chapters: Array<number>` y `Composition.ts` = `"Text" | "Book"`. `pnpm check` falla hasta P4 (literales `Doc` de los tests): se anota y no hay commit intermedio.

### Paso 4 — Frontend y tests de layout
**Archivos:** `src/lib/layout/composition.ts` (nuevo) + `composition.test.ts`, `src/components/Columns.svelte`, `src/lib/layout/anchor.ts` (sólo si muerde el test (h), C8), `tests/layout/defaults.ts`, `tests/layout/{anchor.svelte,columns,images}.test.ts` y `src/lib/layout/{textIndex,hyphenate}.test.ts` (sólo literales `Doc`, C10), `tests/layout/book.test.ts` (nuevo)
- `composition.ts`: `isBookBlank(block)` y `bookClasses(doc)` según C5/C7.
- `Columns.svelte`: `const book = $derived(bookClasses(doc))`; `attrs()` une clases; la rama `Image` añade `book[i]`; `.columns` con `class:book={doc.composition === 'Book'}`; CSS de C6. Nada más cambia en el marcado (una línea, sin nodos entre bloques).
- Unit (`composition.test.ts`): `Text` → todo `''`; `Book` con `[H, blank(" "), P, P, Image, P, LI, LI, Q, P]` → `['', 'blank', 'sep', 'sep indent', 'sep', 'sep', 'sep', '', 'sep', 'sep indent']`; capítulos `[3]` con `[P, P, P, blank(""), P, P]` → el bloque 4 lleva `chapter` sin `sep` ni `indent`, el 5 `sep indent`; `chapters [1, 3]` con `[P, blank, blank, P]` → el bloque 3 lleva `chapter` una sola vez y el 1 no; bloque 0 sin clases; U+00A0 es blanco y `"• "` no.
- Layout (`book.test.ts`, montando `Columns` como `anchor.svelte.test.ts`; libro de prueba = `longDoc` en `Book` con un `Heading` cada 15 párrafos, una imagen `blob:` cada 40, blancos U+00A0 tras cada título y `chapters` cada 60 bloques; replica el stub de `convertFileSrc` y la espera de `img.decode()` de `images.test.ts`, y su `charLeft` salta los bloques `Image` o usa `charRect`; medidas verticales sobre cajas, como en P2):
  - (a) **D9:** `textContent` de cada `<p>` (sin U+00AD) = runs unidos; `getComputedStyle(p, '::before').content` = `none`; último `data-char-start` + longitud = `flatLength`.
  - (b) **Separación:** entre dos `Paragraph` de la misma columna, `top` de la caja del segundo − `bottom` de la caja del primero = línea/2 (±0.5 px); a través de un blanco, también línea/2; el blanco mide 0; en `Text` la misma pareja da 0.
  - (c) **Sangría y títulos:** primer rectángulo de texto de un `Paragraph` con `indent` − `left` de su caja ≈ `1.5 × font-size` (±0.5 px); tras título, tras imagen y en inicio de capítulo ≈ 0; la primera línea del fragmento en la columna siguiente de un párrafo con `indent` partido entre columnas, ≈ 0; el centro del rectángulo de texto de un título = centro de su caja (±1 px).
  - (d) **Sin hueco arriba:** para k = 1..40, el `top` de la caja del primer bloque **no blanco** de la columna k = `vp.top + pad.top` (±1 px).
  - (e) **Capítulos:** para cada `chapters[c]`, el primer bloque no blanco del capítulo empieza en una columna estrictamente posterior a la del último carácter del capítulo anterior, su `top` = `vp.top + pad.top` (±1 px) y, tras `goTo` a esa columna, `firstVisibleCharIndex` = su `data-char-start` y `columnOfCharIndex` de ese índice = esa columna.
  - (f) **Ancla:** `it.each` con los cambios de `anchor.svelte.test.ts` (fuente, ancho, interlineado, insets, escala, ancho del host) más pantalla completa: tras `goTo(37)` el ancla queda en la primera columna visible y `anchor()` no cambia; además con el ancla en el inicio de un capítulo (sangría 0, columna recién abierta) y en un `Paragraph` con `indent` que abre columna. La deriva no se repite (ya cubierta en `Text`; `restore`/`sync` no cambian, C8).
  - (g) **Primer carácter visible:** para k = 1..40, `columnOfCharIndex(firstVisibleCharIndex)` = k (como el test existente, en `Book`).
  - (h) **Ancla sobre un blanco:** `initial_char_index` = inicio de un blanco U+00A0 lejano → tras montar, `anchor()` === `initial_char_index` (restaurar no mueve el ancla) y la columna visible es `columnOfCharIndex(initial_char_index)`, > 0. Si falla, aplicar la corrección de `charRect` de C8 y repetir.
  - (i) **Búsqueda:** `rangeOf` de una coincidencia en un párrafo con `indent` y en un título reproduce el texto; `show()` de una lejana la deja en la columna central.
  - (j) **Paso:** `measurePitch` = `column_width × (1 + 2 × inset%)` (±0.5 px) con un título centrado como primer bloque y con capítulos.
- **Negativas** (cada una se aplica, se ve fallar y se revierte; se anota en §8):
  1. `.book p.sep` con `padding-block-start` en vez de margen → falla (d).
  2. Sin `break-before: column` en `.chapter` → falla (e).
  3. Sin `margin-block-start: 0` en `.chapter` y con `sep` también en el inicio de capítulo → falla el `top` de (e).
  4. `bookClasses` sin saltar blancos al calcular `prev` → falla el unit del párrafo tras título + blanco y (c).
  5. `.book p.blank { display: none }` → falla (h).
  6. Separación por `::before { content: '' ; display: block }` → falla (a).

**Gate:** `pnpm verify` = 0; layout ×3 estable.

### Paso 5 — Comprobación
**Orquestador** (humo con el importador real, test Rust `#[ignore]` local que no se sube, como en el plan EPUB):
- *La acción humana*: `composition == Book`; `chapters` contiene el índice del bloque «INTRODUCCIÓN» y su bloque anterior es la firma del capítulo previo; número de capítulos = ítems del spine con bloques.
- *Libro del desasosiego*: un capítulo por fragmento con bloques; el bloque tras cada título es U+00A0.
- Un `.txt` y el portapapeles → `Text`.
- `pnpm tauri dev`: abrir ambos libros, reabrir en la misma posición, buscar una palabra tardía, Ctrl+S `.txt`.

**Titular** (visual):
- *La acción humana*: «INTRODUCCIÓN» abre columna, arriba, centrada; párrafos con sangría y media línea; el primero tras un título sin sangría.
- *Libro del desasosiego*: comparado con Sumatra (título centrado, sangría, separación); cada fragmento en columna nueva (Q2).
- Un `.txt` con un poema: sin sangría ni separación, como antes.
- Cambiar fuente, ancho, interlineado, escala y pantalla completa conserva el punto de lectura.

**Gate:** confirmación del titular.

### Paso 6 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `DECISIONES.md`, `REGISTRO.md`, este plan
- `ARCHITECTURE.md`: §4 (`layout/composition.ts`), §5 (modelo con `composition` y `chapters`; capítulos no suman texto), §6 (composición de libro: clases, margen truncado, salto de capítulo, blanco de alto 0; títulos centrados), §8 «Fuente aplicada» (títulos centrados, no a la izquierda), §9 fila EPUB (cada ítem del spine con bloques abre capítulo), §12 (`book.test.ts`, premisas de `multicol.test.ts`).
- `DIFERIDOS.md`: fuera la entrada «Composición de libro»; nuevas (sin fase, con fecha y hora): saltos de página de DOCX/RTF como capítulo *(si Q1 = recomendación)*; títulos marcados por clase (`<p class="titulo">`) sin `hN`.
- `DECISIONES.md`: ampliar la línea de «Composición de libro» con Q1–Q3 respondidas («capítulo = ítem del spine EPUB»; blancos invisibles en libro).
- `REGISTRO.md`: asiento. Este plan → `.historial/_PLAN_COMPOSICION_completado.md`.
- Borrar el test de humo `#[ignore]` de P5.
- Commit `feat: composición de libro`.

**Gate:** `pnpm verify` = 0 sobre el commit final; `git status` limpio.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5 ─→ P6
```
P2 decide si C6/C8 valen: nada se construye antes. P3 y P4 forman un bloque sin commit intermedio (`check` rojo entre ambos por los literales `Doc`); si P4 fracasa, se revierten juntos a `f896d60`.

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 sobre `f896d60` | Rust 209, unit 103, layout 38 |
| P2 — Sonda Chromium | ✅ completado | `pnpm test:layout` 44/44 ×3; `check` 0/0 | Negativa padding → falla (a) y (d-1) (real 32, esperado 20). Desvío: la medida vertical es el borde del contenido (`top` del primer fragmento + `padding-block-start`), porque `getClientRects()` incluye el padding y la caja sola no distingue margen de padding. Contenedor de 600 px (con 640 Chromium estira las columnas). (b-2) confirma que el margen se conserva tras salto forzado. |
| P3 — Modelo e importadores | ✅ completado | `lint:rust` 0; `test:rust` 215 (209 + 5 nuevos + `export_bindings` de `Composition`) | Negativas: vacíos empujados → `[2, 2]`; `Book` en texto → fallan `unicode_coincide_con_su_json` y `composicion_segun_formato`; sin copia en `from_bytes` → `[]`. Helpers de test de `epub.rs` en `pub(crate)` para reutilizarlos en `import/mod.rs`. |
| P4 — Frontend y layout | ✅ completado | `pnpm verify` = 0 (Rust 215, check 0/0, unit 109, layout 77); layout ×4 estable | Negativas 1–6 fallan donde predice el plan, revertidas. (h) pasa sin la corrección C8: `anchor.ts` intacto. Desvíos: el archivo es `book.svelte.test.ts` (necesita `$state` para cambiar prefs en vivo, como `anchor.svelte.test.ts`); el libro de prueba añade un blanco entre párrafos cada 25 (sin él no hay caso párrafo–blanco–párrafo para (b)). |
| P5 — Comprobación | ✅ completado | Humo `#[ignore]` (orquestador) en verde; titular conforme en `tauri dev` | *La acción humana*: `Book`, 4.116 bloques, 95 capítulos; «INTRODUCCIÓN» (bloque 137) es capítulo y su anterior es la firma «Universidad Rey Juan Carlos». *Libro del desasosiego* (Calibre): 517 capítulos, uno por fragmento; 514/584 títulos seguidos de U+00A0. Otra edición del *Desasosiego*: 518 capítulos; su primer ítem del spine aporta 522 bloques (notas al frente). `.txt` → `Text`. Falta: `tauri dev` y revisión visual del titular. |
| P6 — Cierre y commit | ✅ completado | `pnpm verify` = 0 sobre el commit final | Humo borrado; documentación y REGISTRO actualizados. |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- Leer la CSS del documento (sangrías, márgenes, alineación, versalitas propias del libro): Miso compone con sus reglas.
- Capítulos en DOCX, RTF, HTML y PDF *(Q1)*; las páginas de un PDF no son capítulos.
- Títulos que el documento no marca como `hN` (DOCX por estilo sí llegan como `Heading`; EPUB con `<p class="…">` no).
- Sangría o separación en `.txt` y portapapeles; cualquier opción en Preferencias.
- Viudas y huérfanas, `break-after: avoid` más allá del actual, espacio extra antes de títulos distinto de su media línea, adornos de cambio de escena.
- Índice navegable, notas enlazadas.

**Auditoría (DeepSeek 4.1, 2026-11-14)**
- Entran: A1 (medidas sobre cajas), A2 (gate de P2 acotado), A3 (sonda del blanco de alto 0), A4 y A8 (rango del capítulo; `chapter` sustituye a `sep`), A5 (test del `Doc` por `from_bytes`), A7 (sin `font-size: 0`), A9 (coordenadas de la sonda), A10 (aserción exacta del ancla), A11 (stub de imágenes en `book.test.ts`), A12 (sin repetir la deriva).
- A6 entra reducido: una aserción en P4 (c), no una sonda en P2. La CSS aplica `text-indent` sólo a la primera línea del bloque, no a cada fragmento; basta con comprobarlo una vez.

**Hallazgos aparcados**
- En *Libro del desasosiego* los títulos llevan `<br>` dentro (`<h2>1<br/>Prefacio</h2>`), que hoy parte el título en dos bloques `Heading`; el segundo sale con `sep` de título. Se observa en P5; si molesta, va a `DIFERIDOS.md`.
- Rendimiento con cientos de capítulos cortos: fase 8.
