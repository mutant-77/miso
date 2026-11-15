# Plan de Implementación: Fase 6 — RTF y PDF

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> C1 **reabre §9 de `ARCHITECTURE.md`** (RTF por RichEdit); aprobada por el titular el 2026-11-13. C2–C9 las fija el orquestador por ser las de menor código; el titular puede reabrirlas antes de implementar.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| C1 | **RTF con un lector propio en Rust (`import/rtf.rs`), no con RichEdit.** Tokenizador de RTF (grupos, palabras de control, `\'hh`, `\uN`) con una pila de estado por grupo. ⚠ Reabre §9. | RichEdit exige `unsafe` COM, un hilo STA con bucle de mensajes y una ventana oculta, y sus viñetas y números de lista son formato de párrafo, no texto: habría que reconstruirlos igualmente. El lector propio ocupa lo mismo, es puro, se prueba con cadenas en cualquier plataforma y los marcadores salen literales de `\listtext`. §9 ya lo preveía como respaldo. | P2 |
| C2 | **RTF con el mismo modelo y el mismo pintado que DOCX** (C1/C2 de la fase 5): estructura + negrita/cursiva/subrayado, fuente, tamaño y colores de prefs. Fuentes, tamaños y colores del documento siguen diferidos, ahora sin fase. | El criterio de §13 es «un RTF con negrita/cursiva abre con sus estilos». El frontend no cambia. | P2 |
| C3 | **Lectura del RTF:** `\par`, `\sect`, `\page` cierran bloque; `\line` lo parte (C3 de la fase 5); `\tab` → `\t`; `\b`, `\i`, `\ul` (con `0` apagan; `\ulnone` apaga), `\plain` reinicia el carácter, `\pard` el párrafo (el estado de párrafo también vive en la pila: el `\pard` que Word pone dentro de `{\listtext …}` no borra `\ilvl` ni `\outlinelevel` del párrafo); el espacio que delimita una palabra de control no es texto; `\outlinelevelN` (0–5) → `Heading { N+1 }`; `{\listtext …}` / `{\pntext …}` → `ListItem { depth: \ilvlN }` con su texto como marcador (`"1. "` si tiene letras o dígitos; si no, `"• "`, porque la viñeta de Word es `\'b7` en la fuente Symbol); tabla: `\cell` cierra celda, `\row` emite un `Paragraph` con las celdas unidas por `\t`, y un `\par` dentro de `\intbl` es un espacio. Se omiten los grupos `{\* …}` y los destinos `fonttbl`, `colortbl`, `stylesheet`, `info`, `pict`, `object`, `header*`, `footer*`, `footnote`, `annotation`, `shp`, `nonshppict`, `fldinst` (LibreOffice lo emite sin `\*`), y el texto oculto (`\v`). De un campo queda `\fldrslt`. `{\pntext …}` es hermano de `{\*\pn …}`, no hijo: sí se lee. | Es lo que emiten Word, WordPad, LibreOffice y TextEdit. Las reglas de tabla, salto y bloque vacío son las de DOCX. | P2 |
| C4 | **Codificación del RTF:** `\ansicpgN` (por defecto 1252) con el crate `codepage` (`to_encoding`, de hsivonen, sobre `encoding_rs`); los bytes `\'hh` y de texto consecutivos se decodifican juntos (páginas de dos bytes) y se vuelcan al texto antes de añadir un `\uN`, así el orden se conserva. `\uN` (negativo → `+65536`; un alto sustituto espera al bajo) salta después los `\ucN` (por defecto 1) caracteres de respaldo siguientes: cada `\'hh` o carácter literal cuenta 1; nunca se salta una palabra de control ni una llave. `\~` → U+00A0, `\-` se descarta, `\_` → `-`, `\emdash`, `\endash`, `\bullet`, `\lquote`, `\rquote`, `\ldblquote`, `\rdblquote` → su carácter; `\binN` salta N bytes. Todo texto pasa por `clean`. | Sin `\uc` el texto de Word sale con caracteres duplicados (`?` de respaldo). `\fcharset` por fuente queda fuera (§9). | P2 |
| C5 | **El RTF nunca es `Corrupt`:** llaves desequilibradas o archivo truncado conservan el texto leído (como RichEdit). Un pánico sigue siendo `Corrupt` por el `catch_unwind` existente. | Un lector tolerante lee más archivos reales; no hay estado del que no se pueda salir. | P2 |
| C6 | **PDF con `pdfium-render` 0.9.5** (`default-features = false`, `features = ["thread_safe", "pdfium_latest"]`: sin el crate `image`) y **`pdfium.dll` x64 de `bblanchon/pdfium-binaries` `chromium/8125`** (la API de `pdfium_latest`), versionada en `src-tauri/pdfium/` con su licencia y `.gitattributes` `binary`. Ruta: en compilación de desarrollo (`debug_assertions`, también `cargo test`), `CARGO_MANIFEST_DIR/pdfium`; en *release*, el directorio del ejecutable (lo empaqueta la fase 7). Una instancia por proceso: `OnceLock<Result<Pdfium, String>>` (`bind_to_library` sólo se llama una vez; la segunda da `PdfiumLibraryBindingsAlreadyInitialized`). | Fija `DECISIONES.md` (PDFium). Versionar la DLL hace la compilación reproducible sin red; el empaquetado es de la fase 7. Con `thread_safe`, `pdfium-render` serializa las llamadas y dos importaciones simultáneas no chocan. | P3 |
| C7 | **Texto del PDF reconstruido en párrafos por geometría, página a página.** Líneas: se parte sólo en los `\r`/`\n` con `is_generated` (PDFium emite el par `\r\n`; los segmentos vacíos se ignoran); los espacios generados se conservan; se descartan U+FFFE/U+FFFF y los caracteres de control salvo `\t`. Caja de línea = unión de los `loose_bounds` (espacio de página: `y` crece hacia arriba). La siguiente línea **sube** si `sig.bottom > act.top` (columna nueva); el **hueco** es `act.bottom − sig.top`. La página se corta en tramos en cada subida y, por tramo: `h` = mediana del alto de línea; `R` = percentil 90 del borde derecho; `L` = percentil 10 del izquierdo; `g` = mediana de los huecos (con menos de 3 líneas: `g = h`, `R` = máximo, `L` = mínimo). Tras una línea hay párrafo nuevo si: termina antes de `R − 2h` de su tramo (línea corta), o el hueco supera `g + 0.5h`, o la siguiente empieza después de `L + h` (sangría). En una subida sólo cuenta la línea corta. Unión: `trim_end` + espacio + `trim_start`; si el último carácter tiene `is_hyphen` y la siguiente empieza en minúscula, se quita y se une sin espacio. Cada página cierra su párrafo. Todo es `Paragraph` sin estilos. | Sin reconstrucción, cada línea del PDF sería un párrafo justificado en una columna de 360 px: ilegible. El orden lo da PDFium; las reglas usan sólo medidas relativas de la propia página. Títulos, estilos, cabeceras y párrafos entre páginas quedan fuera (§9). | P3 |
| C8 | **Detección (antes que la actual):** empieza por `%PDF-` → PDF; empieza por `{\rtf` → RTF; después `PK`, HTML por extensión y texto (C4 de la fase 5). | Ambos se reconocen por contenido, como Tofu, que prueba PDF primero (evidencia §9.1). | P2, P3 |
| C9 | **Errores:** variante nueva `AppError::Protected { file }` («No se pudo abrir «{file}»: está protegido con contraseña.», `kind` `"protected"`) para `PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError)`; otro fallo al cargar el PDF → `Corrupt`; un PDF sin texto (escaneado) → `Unsupported`; la DLL que no carga → `Internal` (instalación rota; detalle al log). `ParseError` gana `Protected` e `Internal(String)`. | Cierra el diferido «PDF con contraseña». Mensajes en el único `match` de `user_message()` (§5). | P3 |
| C10 | **Diálogo Ctrl+O:** el filtro «Texto» (`txt`) pasa a «Documentos» (`txt`, `rtf`, `pdf`, `html`, `htm`, `xhtml`, `docx`); «Todos» sigue. | Hoy el diálogo sólo ofrece `.txt` aunque la fase 5 ya abre HTML y DOCX. | P3 |

## 2. Objetivo

Criterio de hecho de §13 (fase 6): **un RTF con negrita/cursiva abre con sus estilos; la mayoría de PDF de texto se lee con orden razonable.**

## 3. Resumen del análisis

- **Stack:** Rust 2024 + Tauri 2, `ts-rs` 12. Dependencias nuevas: `codepage` (MIT/Apache), `pdfium-render` 0.9.5 (MIT/Apache); binario `pdfium.dll` (BSD-3/Apache, `bblanchon/pdfium-binaries`).
- **Estado heredado (`7c3f555`):**
  - `import/mod.rs::parse` despacha `PK` → `docx`, extensión HTML → `html`, resto → `plain`; `from_bytes` envuelve en `catch_unwind` y traduce `ParseError { Unsupported, Corrupt }` a `AppError`. `clean` normaliza el texto de los importadores ricos.
  - `docx.rs` fusiona runs contiguos del mismo estilo en `push_text` (líneas ~335–350): se mueve a `import::push_run` y lo usan DOCX y RTF.
  - El modelo (`Block`, `Run`) ya cubre todo lo que necesita esta fase: **el frontend no cambia**.
  - `commands.rs::open_dialog` filtra «Texto» (`txt`) y «Todos».
  - API de `pdfium-render` 0.9.5 comprobada en su código: `PdfPageText::chars()`, `PdfPageTextChar::{unicode_char, loose_bounds, is_generated, is_hyphen}`, `Pdfium::{bind_to_library, pdfium_platform_library_name_at_path, load_pdf_from_byte_slice}`, `PdfiumInternalError::PasswordError`. Las *bindings* viven en un `OnceCell` global del crate.
- **Cobertura:** Rust 118, unit 98, layout 32.
- **Riesgos:**
  1. Párrafos de PDF: las reglas de C7 son heurísticas; maquetaciones raras (poesía, tablas, dos columnas con notas) pueden partir o unir de más. Se acepta: el criterio es «la mayoría».
  2. Rendimiento: una llamada FFI por carácter. Un PDF de 500 páginas puede tardar segundos (corre en `spawn_blocking`, la UI no se bloquea). Medición en la fase 8.
  3. Tamaño del repositorio: la DLL añade ~5 MB a git, una vez.

## 4. Restricciones innegociables

- Nunca `pnpm build`, `pnpm tauri build` ni instaladores. `pnpm tauri dev` sí. Puertas desde PowerShell.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D2, D3/D9 (índices UTF-16; todo texto por `clean`; ningún `\n` dentro de un bloque), D10, D11 (el frontend no muestra errores).

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos (`import/{mod,plain,docx}.rs`, `model.rs`, `error.rs`, `commands.rs`, `Cargo.toml`, `tauri.conf.json`, `.gitattributes`)
- [x] Evidencia de Tofu: §9.1 (PDF primero; `continuousAttributedString`)
- [x] API de `pdfium-render` 0.9.5 comprobada en el registro local de Cargo
- [x] Aprobación del titular de C1 (2026-11-13: menos sobrecarga que RichEdit)
- [x] Fase 5 confirmada por el titular (P6: F11 funciona, 2026-11-13)

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- `git status` limpio salvo este plan; `pnpm verify` en verde. Punto de vuelta atrás: `7c3f555`.

**Gate:** `pnpm verify` = 0 sobre `7c3f555`.

### Paso 2 — Importador RTF
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/src/import/{mod,docx}.rs`, `src-tauri/src/import/rtf.rs` (nuevo)
- `import::push_run(runs, text, bold, italic, underline)`: la fusión de `docx.rs`, movida sin cambiar comportamiento (aplica `clean` y descarta el texto vacío).
- `rtf::parse(bytes) -> Vec<Block>` (C1, C3–C5): un solo recorrido de bytes; pila de estado por grupo (`bold`, `italic`, `underline`, `hidden`, `uc`, destino omitido, `intbl`, `outline`, `ilvl`, dentro de `listtext`); un búfer de bytes pendientes (`\'hh` y texto) que se decodifica al llegar un `\uN`, al cambiar de estilo o al cerrar bloque (C4). Al final se emite el párrafo pendiente si tiene texto (un `\par` final no deja bloque vacío); un `\par` sin texto en medio sí (línea en blanco, como DOCX).
- `mod.rs`: rama `{\rtf` → `rtf::parse` (C8).
- Tests (cadenas en el propio test, comparando texto aplanado y estilos): `\b`/`\b0`/`\i`/`\ul`/`\ulnone`/`\plain`; `\par` y `\line` (ningún run con `\n`); `\'e9` con 1252 y `\'cf` con `\ansicpg1251`; `a\u233 ?b` → `aéb` (orden y delimitador); `\u-10179\u-8704` (par sustituto) con `\uc1` y su `?` de respaldo omitido; `\uc2` con dos `\'hh` de respaldo; `\uc0`; destinos omitidos (`fonttbl`, `colortbl`, `stylesheet`, `info`, `{\*\generator}`, `pict` con `\bin`, `{\fldinst …}` sin `\*`); `\outlinelevel0` → `Heading 1`; `{\listtext\pard\plain\'b7\tab}` → `"• "` y `{\listtext 1.\tab}` con `\ilvl1` → `"1. "` y `depth 1` (el `\pard` interior no borra el `\ilvl` exterior); `{\pntext\'b7\tab}{\*\pn …}` de WordPad → `"• "`; tabla de dos celdas con un `\par` en una celda; `\{`, `\}`, `\\`, `\~`, `\-`, `\_`, `\emdash`; campo con `\fldrslt`; `\v` oculto; RTF truncado sin llaves de cierre conserva el texto; un RTF real de WordPad pequeño (literal en el test). La fusión de DOCX sigue verde sin tocar sus tests.

**Gate:** `pnpm verify` verde.

### Paso 3 — Importador PDF
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/pdfium/{pdfium.dll,LICENSE}` (nuevos), `.gitattributes`, `src-tauri/src/import/{mod,pdf}.rs` (`pdf.rs` nuevo), `src-tauri/src/error.rs`, `src-tauri/src/commands.rs`, `tests/fixtures/protegido.pdf` (nuevo)
- Descargar una vez `pdfium-win-x64.tgz` de `chromium/8125` y copiar `bin/pdfium.dll` y `LICENSE` a `src-tauri/pdfium/` (C6). `.gitattributes`: `*.dll binary`, `*.pdf binary`.
- `pdf.rs`: `pdfium() -> Result<&'static Pdfium, String>` sobre el `OnceLock` de C6 (si la DLL no carga, `ParseError::Internal`). `parse(bytes) -> Result<Vec<Block>, ParseError>`: `load_pdf_from_byte_slice(bytes, None)`; `PasswordError` → `Protected`; otro error → `Corrupt`. Por página, líneas y párrafos según C7; los caracteres de control (salvo `\t`) se descartan antes de `clean`. Sin texto en todo el documento → `Unsupported`.
- `mod.rs`: rama `%PDF-` (C8); `ParseError::{Protected, Internal}` → `AppError`.
- `error.rs`: `Protected { file }` (C9), en `variants()` y `serializa_a_kind_y_message`.
- `commands.rs`: filtro «Documentos» (C10).
- Tests: los PDF se generan en el test con el propio `pdfium-render` (`create_new_pdf`, objetos de texto en Courier en posiciones fijas, `save_to_bytes`), sin binarios salvo `protegido.pdf`: dos párrafos separados por hueco → 2 bloques; última línea corta → corte; sangría de primera línea → corte; `pala-` + `bra` → `palabra`; guion seguido de mayúscula se conserva; dos columnas de un solo párrafo cada una → la columna izquierda entera es **un** bloque y la subida no lo corta; dos páginas → al menos dos bloques; PDF sin texto → `Unsupported`; `%PDF-1.7 basura` → error (`Corrupt`, o `Unsupported` si PDFium lo repara vacío), nunca pánico; `protegido.pdf` → `Protected`; ningún run con `\n` ni U+FFFE. `protegido.pdf`: una página con texto y contraseña de usuario, generada una vez con `qpdf --encrypt` o `pikepdf` y versionada.

**Gate:** `pnpm verify` verde.

### Paso 4 — Comprobación en la app
**Archivos:** ninguno (orquestador, `pnpm tauri dev`; el orquestador no verifica a ojo, lo visual es del titular).
- Log sin errores al abrir un `.rtf` y un `.pdf`; la DLL carga desde `src-tauri/pdfium` en desarrollo.

**Gate:** log limpio y DLL cargada; el resto, confirmación del titular.
- **Titular:** un `.rtf` guardado con Word o WordPad abre con títulos, negritas, cursivas, listas y tildes; un PDF de texto (artículo o libro) se lee en párrafos y en orden; un PDF con contraseña muestra su aviso; un PDF escaneado muestra «no es un archivo de texto»; Ctrl+O ofrece «Documentos»; reabrir un PDF vuelve al punto.

### Paso 5 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, este plan
- Pruebas negativas (revertidas): sin el salto de `\ucN` falla su test; sin la regla de línea corta falla el de corte de párrafo.
- `ARCHITECTURE.md`: §4 (`import/rtf.rs` sin RichEdit, `src-tauri/pdfium/`), §5 (`Protected` en la tabla; `Corrupt` sin el hilo STA; importadores en `spawn_blocking` sin excepción), §9 (detección C8; tabla de importadores: RTF propio según C1/C3/C4, PDF según C6/C7; fuera la línea «RTF y PDF al final»), §12 (RTF con cadenas, PDF generados en el test), §13 (fase 6 hecha y «RTF (RichEdit)» → «RTF propio»).
- `DIFERIDOS.md`: fuera «PDF con contraseña»; «Fuentes, tamaños y colores» pasa a «Sin fase»; nuevos: PDF con títulos, estilos, cabeceras/pies y párrafos que cruzan página (sin fase); RTF con `\fcharset` por fuente (sin fase); `pdfium.dll` en el empaquetado (fase 7).
- `REGISTRO.md`: asiento de fase 6. Este plan → `.historial/_PLAN_FASE_6_completado.md`.
- Commit `feat: fase 6 — RTF y PDF`.

**Gate:** `pnpm verify` verde sobre el commit final; `git status` limpio.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5
```
P3 tras P2 porque ambos tocan el despacho de `mod.rs`; pueden ir en paralelo si el orquestador fusiona esa rama.

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 (Rust 118, unit 98, layout 32) | |
| P2 — Importador RTF | ✅ completado | 29 tests RTF; negativa: sin el salto de `\ucN` fallan 3 | Revisión: el salto de `\ucN` no cruza llaves; `\line` en un elemento de lista sigue siendo lista (como DOCX) |
| P3 — Importador PDF | ✅ completado | `pnpm verify` = 0 (Rust 158); negativa: sin la regla de línea corta fallan 2 | Desvío en C7 (guion U+0002); test de columnas según el plan |
| P4 — Comprobación en la app | ✅ orquestador | `tauri dev` con un `.rtf` y un `.pdf`: ambos abren, sin errores de PDFium | Lo visual queda para el titular |
| P5 — Cierre y commit | ✅ completado | `pnpm verify` = 0 | |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- RichEdit y RTFD (C1).
- Fuentes, tamaños, colores y alineación del documento (C2).
- RTF: `\fcharset` por fuente, imágenes, notas, comentarios, cabeceras/pies, cuadros de texto, tablas anidadas (su texto queda en la celda), citas.
- PDF: títulos, negrita/cursiva, cabeceras y pies de página, párrafos que cruzan página, notas, OCR de escaneados, formularios y anotaciones, otras arquitecturas que x64.
- RTF desde el portapapeles (`CF_RTF`).
- Empaquetado de `pdfium.dll` (fase 7).

**Hallazgos aparcados**
- (vacío)

**Desvíos de la implementación (2026-11-13)**
- C7, guion: PDFium no emite `-` + `\r\n` generado en un guion de fin de línea. Funde el salto en un U+0002 con `is_hyphen` y las dos líneas quedan en una. Comprobado por el agente con `pala-`/`bra` y `Norte-`/`Sur`. Un glifo con `is_hyphen` se trata como `-` + fin de línea, y las reglas de C7 se aplican igual. Sin esto, `Norte-` + `Sur` daba `NorteSur` y la línea fundida medía dos renglones. Pendiente de ratificación del titular.
- Test de columnas: el implementador lo había adaptado para que las dos columnas se fundieran en un bloque. Se rehízo como pide el plan (columnas de un párrafo cada una, que acaban en línea corta → 2 bloques). Se añadió además el caso de un párrafo que continúa en la otra columna (1 bloque, porque en una subida sólo cuenta la línea corta).
- RTF: un sustituto alto sin pareja sale como U+FFFD; cada celda se recorta por la derecha; test añadido con `\ansicpg932` para los caracteres de dos bytes.
- `protegido.pdf`: no había `qpdf` ni `pikepdf`; se generó con `pypdf` (RC4-128, contraseña de usuario `miso`).

## 10. Auditoría

2026-11-13, agy y OpenCode (Big Pickle), sólo lectura y sin respaldo físico; deliberación del orquestador (Opus).

- **Aplicado:** espacio delimitador tras `\uN` y `\ucN` que cuenta escapes, nunca palabras de control (C4, con tests de `\uc2` y del orden); un solo búfer de bytes que se vuelca antes de cada `\uN` (orden); `fldinst` sin `\*` omitido; `\pard` dentro de `\listtext` contenido por la pila; coordenadas de página explícitas (`y` hacia arriba) y estadísticas por tramo entre subidas (con `R` global, toda la columna izquierda de un PDF a dos columnas eran líneas cortas); corte sólo en `\r`/`\n` generados, espacios generados conservados, U+FFFE descartado y guion por `is_hyphen`; unión con recorte (sin espacios dobles); páginas de menos de 3 líneas; `OnceLock<Result<…>>` (`get_or_init` no propaga errores); ruta exacta del `PasswordError`; `protegido.pdf` versionado, sin rama de omisión; test de `basura` sin suponer que PDFium no repara; gate de P4; líneas obsoletas de §9 y §13.
- **Simplificado:** detección por prefijo (`%PDF-`, `{\rtf`), sin buscar en el primer KB; fuera `-` y U+0002 como marcas de guion.
- **Rechazado:** `{\pntext}` inalcanzable (Big Pickle): en WordPad es hermano de `{\*\pn}`, no hijo; se mantiene con test. PDFs de test como fixtures binarios (agy): habría que generarlos con alguna herramienta igualmente, y generados en el test documentan su geometría; sólo `protegido.pdf` es fixture porque PDFium no cifra.
