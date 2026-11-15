# Auditoría — ARCHITECTURE.md

**Fecha:** 2026-10-20 · **Alcance:** coherencia interna, marcado `(verificar)`, huecos frente a `rea-tofu-evidence.md` y a las reglas de `AGENTS.md`.
**Veredicto:** base sólida y bien razonada (D1–D9, paso de columna medido, índice UTF-16, clave NTFS). Hay 3 erratas, 4 incoherencias, 6 huecos relevantes y 4 riesgos sin marcar `(verificar)`. Ninguno obliga a rehacer la arquitectura.

Severidad: **A** bloquea o rompe una fase · **M** genera retrabajo · **B** cosmético o de claridad.

---

## 1. Erratas

| # | Sev | Dónde | Problema | Cambio propuesto |
|---|---|---|---|---|
| E1 | B | §3, fila `TofuWindowController` | Ruta `src/lib/reader/Reader.svelte`; en §4 vive en `src/components/Reader.svelte`. | Sustituir por `components/Reader.svelte`. |
| E2 | B | §4, `tests/e2e/` | "ver sección 10" (Búsqueda). E2E está en §12. | "ver sección 12". |
| E3 | B | §5, tabla de comandos | `get_prefs / set_prefs` comparten entrada "`Prefs` parcial"; `get_prefs` no recibe nada. | Separar en dos filas: `get_prefs` — / `Prefs`; `set_prefs` `Prefs` parcial / `Prefs`. |

## 2. Incoherencias

| # | Sev | Dónde | Problema | Cambio propuesto |
|---|---|---|---|---|
| I1 | M | §5 Modelo `Run.size_pt` vs D8 | D8 prohíbe `pt`; el campo se llama `size_pt` y no se define su conversión. | Renombrar a `size_px` y añadir: "Rust convierte el tamaño declarado (RTF/DOCX: medios puntos) a px con la regla de D8". |
| I2 | M | §9 "El orden se mantiene: texto plano y HTML primero, RTF y PDF al final" | Justo debajo de "prueba PDF, luego rico, luego texto": parece contradecir el orden de detección. Es orden de **implementación**. | "Orden de implementación (ver §13): texto plano y HTML primero, RTF y PDF al final." |
| I3 | B | §13 fase 6 | "según la decisión de crate": la decisión ya está cerrada (§9, §14.2). Criterio de hecho solo cubre PDF. | "RTF (RichEdit) y PDF (PDFium)" · criterio: "un RTF con negrita/cursiva y la mayoría de PDF de texto se leen con orden razonable". |
| I4 | M | §10 "El temporizador de 1 s de fin de tecleo se conserva" | En Tofu ese temporizador solo sirve para concatenar en `NSPasteboardNameFind`, que se descarta. Se conserva un mecanismo sin función. | Darle función explícita ("debounce de 1 s antes de buscar") o eliminar la frase. |
| I5 | M | §14 "Decisiones cerradas" vs `DECISIONES.md` (vacío) | `AGENTS.md` asigna las decisiones del titular a `DECISIONES.md`. Hoy viven en ARCHITECTURE y no quedan protegidas frente a auditorías. | Con aprobación del titular: mover §14.1–14.4 a `DECISIONES.md` (una línea cada una) y dejar en §14 una referencia. |

## 3. Huecos relevantes

| # | Sev | Hueco | Por qué importa | Cambio propuesto |
|---|---|---|---|---|
| H1 | **A** | **Entrega del `Doc` a una ventana nueva.** §5 solo ofrece el evento `document-opened`. | Rust crea la ventana y emite antes de que el WebView haya registrado `listen()`: el evento se pierde (carrera clásica en Tauri). | Modelo *pull*: Rust guarda el `Doc` pendiente por `label`; la ventana, al montar, llama a `take_pending_document()`. El evento queda solo para ventanas ya vivas. Añadir el comando a la tabla. |
| H2 | **A** | **`doc_key` no tiene origen.** `get_position(doc_key)` existe, pero `Doc` no lo lleva y el frontend no puede calcular `file_id`. | El contrato IPC no es implementable tal cual. | Que `open_document` resuelva la posición en Rust: `Doc { …, doc_key?, initial_char_index? }`. `set_position(doc_key, char_index)` se mantiene. `get_position` puede eliminarse. |
| H3 | **A** | **Las pruebas críticas de layout no tienen dónde correr.** Fase 2 exige "500 saltos sin error" y "conserva el primer carácter"; §12 pone esto en vitest, que (jsdom) no maqueta, y deja E2E como opcional. | El criterio de hecho de la fase 2 es inverificable con la pila declarada. | §12: "Tests de layout (`pitch`, ancla, 500 saltos) en **vitest browser mode** con Chromium (Playwright) o `tauri-driver`; no son opcionales". |
| H4 | M | **Atajos de aplicación y menú.** §7 solo cubre navegación. Faltan: abrir (Ctrl+O), pegar como documento (Ctrl+V, `TofuTextView paste:`), buscar (Ctrl+F, F3), preferencias (Ctrl+,), pantalla completa (F11/Esc), **tamaño de fuente (Ctrl+ / Ctrl−)**. | Tofu tiene `increaseFontSize:`/`decreaseFontSize:`/`mostUsedFontSize`; no aparecen en ninguna fase. | Añadir §7.1 "Atajos de aplicación" y una línea en §8 sobre cómo interactúan `FontSize` de prefs, `is_plain`, `ForcePreferredFont` y los tamaños del documento (Tofu: el texto plano usa la fuente de prefs; el rico conserva la suya salvo `ForcePreferredFont`). Asignar a fase 2. |
| H5 | M | **Mismo archivo abierto dos veces.** No se dice qué pasa. | `NSDocument` enfoca la ventana existente; sin regla, dos ventanas escriben posiciones del mismo documento. | §9: "Si el documento ya está abierto (mismo `doc_key`), se enfoca su ventana". |
| H6 | M | **Contenido no representado.** `Doc` no tiene imágenes, tablas ni enlaces, y no se dice. | Sin explicitarlo, cada importador decidirá distinto (texto alternativo, celdas, URL). | §5: "Imágenes se omiten; tablas se aplanan a párrafos por celda/fila; enlaces se pintan como texto (sin `opener`)". |

## 4. Riesgos técnicos sin marcar `(verificar)`

| # | Sev | Riesgo | Cambio propuesto |
|---|---|---|---|
| R1 | **A** | **Drop de texto vs drop de archivos en Windows.** En Tauri 2, con `dragDropEnabled: true` (necesario para recibir rutas) el WebView2 desactiva el drag & drop HTML5: no se puede soltar texto. Choca con `source: Drop` y §9.2. | Marcar `(verificar)` en §9.2 y decidir: o solo archivos, o desactivar el nativo y leer `DataTransfer` en el frontend (perdiendo rutas absolutas). |
| R2 | M | **RichEdit/TOM y los hilos.** COM y los controles Win32 exigen un hilo STA con su propia ventana; los comandos de Tauri corren en un pool. | Marcar `(verificar)`: hilo dedicado STA para `rtf.rs`, o RichEdit sin ventana (`CreateTextServices`). Añadir en §5: "los importadores corren fuera del hilo principal (`spawn_blocking`)". |
| R3 | M | **Seguridad de rutas.** `open_document(path)` y `save_copy(…, path)` aceptan cualquier ruta del frontend; en Tauri los comandos propios no pasan por el *scope* de capabilities, así que §11 ("lectura del archivo elegido") no se cumple por sí solo. | Abrir diálogos y recibir drops **en Rust** (`WindowEvent::DragDrop`), de modo que el frontend nunca envía rutas. Elimina además el permiso `dialog` del frontend. |
| R4 | M | **Respaldo de la clave de documento.** El caso "editor que guarda escribiendo un archivo nuevo y renombrándolo" cambia el `file_id` **y** el contenido, así que el hash tampoco coincide. `path` solo desempata. | Orden de búsqueda: `file_id` → `content_hash` → **`path` exacta**. Añadir: en ReFS el ID es de 128 bits (`GetFileInformationByHandleEx` + `FileIdInfo`). |

## 5. Menores

- **`lang` no está en `Doc`**, pero §6 lo exige para `hyphens: auto`. Añadir `lang?` al modelo (DOCX `w:lang`, HTML `lang`).
- **`ammonia` sobra**: sanea HTML para reinyectarlo, y §11 prohíbe inyectarlo. `scraper` basta. Igual en el comentario de `html.rs` ("sanitiza").
- **`windows.rs`** colisiona con el crate `windows` en `lib.rs` (`use windows::…` es ambiguo, E0659). Renombrar a `doc_windows.rs`.
- **Rueda vertical**: en un contenedor con `overflow-x` el navegador no convierte `deltaY` en scroll horizontal. Decirlo en `wheel.ts`.
- **`↑/↓` media columna** se aparta de Tofu (línea dentro de la columna). Marcar como desviación deliberada.
- **`padding-block` con `height: 100%`** requiere `box-sizing: border-box` en `.columns`.
- **Versión mínima de WebView2** (§10) debe cubrir también `caretPositionFromPoint` (Chromium 128), no solo Highlight API (105).
- **Exportación**: Tofu guarda RTFD; se descarta en silencio. Indicarlo en §1 "fuera de alcance".
- **Sin definir**: rangos válidos de `ensureValidColumnWidth`, idioma de la UI, y cómo se muestran los `AppError` al usuario.

## 6. Lo que está bien y no debe tocarse

D1 (multicol), D3+D9 (ancla en UTF-16 con reglas de aplanado y test cruzado), el paso de columna medido en DOM con ancho exacto en ventana normal, la regla √2 verificada, la clave NTFS con respaldo, y el marcado `(verificar)` ya presente en tipos generados, partición de palabras, rendimiento e importadores.

## 7. Orden sugerido de corrección

1. H1, H2, R1 (contrato IPC y entrada; afectan a fases 1–3).
2. H3 (sin esto la fase 2 no se puede cerrar).
3. I5 (mover decisiones a `DECISIONES.md`, requiere aprobación del titular).
4. Resto en una sola pasada de edición.
