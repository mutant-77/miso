# Plan de Implementación: Fase 7 — Exportar copia y ejecutable propio

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco. Tampoco se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> C7 y C8 **reabren §13** (fase 7: «instalador funcional»); aprobado por el titular el 2026-11-14: Miso no se distribuye, así que no hay instalador. C1–C6 las fija el orquestador por ser las de menor código.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| C1 | **`save_copy` sin argumentos.** Rust abre el diálogo de guardado (filtros «Texto» `txt` y «RTF» `rtf`, colgado de la ventana, nombre propuesto = título **sin extensión**: `rfd` fija `txt` como extensión por defecto y Windows pone la del filtro elegido; con `.txt` escrito, elegir «RTF» seguiría guardando `.txt`) y el formato sale de la extensión devuelta: `rtf` (sin distinguir mayúsculas) → RTF; cualquier otra o ninguna → texto. Ventana sin documento: no hace nada. Cancelar no es error. | Es el panel simple de Tofu (`shouldRunSavePanelWithAccessoryView` = NO, evidencia §9.7). El diálogo no devuelve el filtro elegido, la extensión sí. Cambia la firma de §5 (`format` sobraba). | P2 |
| C2 | **Rust conserva el `Doc` de cada ventana:** `Slot::Pending` y `Slot::Open` llevan `(Doc, Option<DocKey>)`. `take` sigue entregando sólo la primera vez (de `Pending` a `Open`, devuelve un clon); `Slots::doc(label)` devuelve un clon en cualquiera de los dos estados, para `save_copy`. | §5 ya lo exige («lo necesitan `save_copy` y la regla 1»), pero el código lo suelta al entregarlo. La entrega única de `take` no cambia. | P2 |
| C3 | **Un solo módulo `export.rs`** con `plain(&Doc) -> String` y `rtf(&Doc) -> String`, funciones puras. Sustituye a `export/{mod,plain,rtf}.rs` de §4. | Son dos funciones cortas; tres archivos serían ceremonia. | P2 |
| C4 | **Texto:** UTF-8 sin BOM, un bloque por línea, unidas por `\r\n`, sin salto final. El texto de los runs tal cual (los marcadores de lista ya son texto). | Convención de Windows (D7); el importador de texto lo reabre con el mismo texto (`plain.rs` normaliza `\r\n`). Límite aceptado: 8+ párrafos seguidos de longitud parecida pueden reabrirse unidos por la heurística de texto cortado a ancho fijo (mismos caracteres, salvo `\n` → espacio). | P2 |
| C5 | **RTF (ASCII puro, legible por Word y nuestro `rtf.rs`):** `{\rtf1\ansi\ansicpg1252\deff0\uc1{\fonttbl{\f0\froman Times New Roman;}}`, los bloques y `}` final. Cada bloque: `\pard\plain` (el `\pard` no reinicia la negrita: sin `\plain`, la de un título pasa a los bloques siguientes) + formato + espacio + runs + `\par` y salto de línea de fuente. Formato: `Heading` de nivel `n` (1–6) → `\outlinelevel` con `n−1` y `\b` (p. ej. `\pard\plain\outlinelevel0\b Título\par`); `ListItem` de profundidad `d` → `\li` con `360 × (d+1)` (p. ej. `\li360 • Uno`), marcador como texto; `Quote` → `\li720\ri720`; `Paragraph` → nada. Run con estilo → `{\b\i\ul texto}` (sólo los que tenga, espacio tras la última palabra de control). Escapes: `\` `{` `}` → `\\` `\{` `\}`; `\t` → `\tab `; U+00A0 → `\~`; cualquier otro fuera de 0x20–0x7E → `\uN?` por unidad UTF-16 (`N` como `i16`). | Sin tabla de codificación que mantener. Fuentes, tamaños y colores del documento siguen diferidos. Al reabrirlo en Miso, títulos y estilos vuelven (los runs de un título, en negrita); listas y citas vuelven como párrafos con el mismo texto (sin `\ls` de Word no hay lista que declarar). | P2 |
| C6 | **Error nuevo `AppError::Write { file, source }`**: «No se pudo guardar «{file}»: {os_message}», `kind` `"write"`, con `file = import::file_name(&path)` (nunca la ruta). Escritura directa (`fs::write`) en `spawn_blocking`. | Cierra la fila `Write` de §5. Es una copia; el diálogo ya confirma la sobrescritura. | P2 |
| C7 | **Bundle id `com.mutant77.miso`** (titular, 2026-11-14). Cambia el directorio de datos: prefs y posiciones del id provisional no se migran. | Lo exigía `DECISIONES.md` antes de esta fase. Sin usuarios fuera del desarrollo. | P3 |
| C8 | **Sin instalador: ejecutable de *release* para uso propio.** `bundle.active: false` (`pnpm tauri build` sólo compila el `.exe`). Icono: `assets/miso.svg` (del titular) → `pnpm tauri icon assets/miso.svg`; `bundle.icon` sigue listando los de escritorio (el `.ico` va dentro del `.exe`). **`pdfium.dll` va dentro del `.exe`** (`include_bytes!` de `pdfium/pdfium.dll`); al iniciar PDFium (una vez por proceso, ya dentro del `OnceLock`) se escribe en `temp_dir()/miso-pdfium-8125.dll` si no existe o su tamaño no coincide (escritura a `.tmp` + `rename`) y se carga de ahí con `bind_to_library`. Un solo camino en desarrollo, tests y *release*; fuera `directorio()` y su `cfg!(debug_assertions)`. | Miso no se distribuye, pero el `.exe` queda autónomo: se copia a cualquier carpeta sin repositorio ni red. Windows no carga DLL desde memoria; extraerla una vez es lo mínimo. La carpeta temporal no necesita `AppHandle` y vale también en `cargo test`; si se limpia, se reescribe. La versión en el nombre evita cargar una DLL vieja. | P3 |

## 2. Objetivo

Ctrl+S guarda una copia en texto o RTF, y el `.exe` de *release* funciona fuera de `tauri dev` con su icono, su id y PDF.

## 3. Resumen del análisis

- **Estado heredado (`6ecffda`):**
  - `doc_windows.rs`: `Slot::Open(Option<DocKey>)` no guarda el `Doc` (C2). `take`, `key`, `close` y sus tests.
  - `commands.rs::open_dialog` es la plantilla de `save_copy` (diálogo en `spawn_blocking`, `join_result`, `report`).
  - `error.rs`: falta `Write`; `variants()` y `serializa_a_kind_y_message` enumeran todas las variantes.
  - `keyboard.ts` no tiene acción de guardado; `Reader.svelte` despacha acciones en un `switch`.
  - `import/rtf.rs` lee lo que C5 escribe (`\outlinelevel`, `\b\i\ul`, `\uN` con `\uc1`, `\tab`, `\~`): sirve de comprobación de ida y vuelta.
  - `tauri.conf.json`: id `com.provisional.miso`, `targets: "all"`, iconos por defecto de Tauri. `pdf.rs::directorio()`: en desarrollo `CARGO_MANIFEST_DIR/pdfium`, en *release* sólo la carpeta del ejecutable (allí no hay DLL). `pdfium()` ya inicia una sola vez por proceso (`OnceLock`).
- **Riesgos:**
  1. El orquestador no compila *release* (`AGENTS.md`): la comprobación del `.exe` es del titular.
  2. CSP: Svelte aplica `style:` por CSSOM (`setProperty`, `cssText`), que la CSP no bloquea; se confirma en P4. Si hubiera violaciones, se añade sólo `style-src 'self' 'unsafe-inline'` (no abre red, D6).

## 4. Restricciones innegociables

- El orquestador nunca ejecuta `pnpm build` ni `pnpm tauri build`; `pnpm tauri dev` y `pnpm tauri icon` sí. Puertas desde PowerShell.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo.
- D3/D9, D10 (el frontend no envía rutas: `save_copy` no recibe nada), D11 (el frontend no muestra errores).

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos (`doc_windows.rs`, `commands.rs`, `error.rs`, `model.rs`, `lib.rs`, `import/pdf.rs`, `keyboard.ts`, `commands.ts`, `Reader.svelte`, `tauri.conf.json`)
- [x] Evidencia de Tofu: §9.7 (guardado: texto y RTF, panel simple)
- [x] Dominio (`mutant77`) e icono (`assets/miso.svg`) del titular

## 6. Pasos secuenciales

### Paso 1 — Línea base
**Archivos:** ninguno.
- `git status` limpio salvo este plan y `assets/miso.svg`; `pnpm verify` en verde. Punto de vuelta atrás: `6ecffda`.

**Gate:** `pnpm verify` = 0 sobre `6ecffda`.

### Paso 2 — Exportar copia (Ctrl+S)
**Archivos:** `src-tauri/src/{doc_windows,commands,error,lib}.rs`, `src-tauri/src/export.rs` (nuevo), `src/lib/ipc/commands.ts`, `src/lib/nav/keyboard.ts` (+ test), `src/components/Reader.svelte`
- `doc_windows.rs`: C2. Los tests existentes siguen verdes.
- `export.rs`: C3–C5.
- `error.rs`: C6.
- `commands.rs`: `save_copy` (C1, C6) sobre el patrón de `open_dialog`; registrado en `lib.rs`.
- Frontend: acción `save` en Ctrl+S (`s`/`S`), `saveCopy()` en `commands.ts`, caso en `Reader.svelte` como `open`.
- Tests Rust: texto con CRLF y sin salto final; RTF escapa `\ { }`, tabulador, U+00A0, un control (`\u{1}`), `ñ`, CJK y emoji (par sustituto `\u-10179?\u-8704?`); **ida y vuelta** `rtf::parse(export::rtf(doc))` con un título seguido de un párrafo sin estilo: mismo texto aplanado, mismos estilos por run (los del título, en negrita en el fixture), mismos niveles de título y el párrafo siguiente sin negrita; lista y cita vuelven como `Paragraph` con su texto; párrafo vacío intermedio se conserva. `Slots::doc` sobrevive a `take` y desaparece con `close`. `Write` en `variants()` y en la tabla de serialización. Test TS: Ctrl+S → `save`.

**Gate:** `pnpm verify` verde.

### Paso 3 — Id, icono y DLL
**Archivos:** `src-tauri/tauri.conf.json`, `src-tauri/icons/*` (regenerados), `src-tauri/src/import/pdf.rs`, `DECISIONES.md`
- `identifier: "com.mutant77.miso"` y `bundle.active: false` (C7, C8); `DECISIONES.md` fija el id y «sin instalador».
- `pnpm tauri icon assets/miso.svg`; se borran las carpetas de móvil si las genera.
- `pdf.rs`: DLL embebida y extraída según C8. Test: tras abrir un PDF, el archivo extraído existe y mide lo mismo que el embebido.

**Gate:** `pnpm verify` verde; `pnpm tauri dev` arranca, abre un PDF (con `miso-pdfium-8125.dll` en `%TEMP%`) y crea `%APPDATA%\com.mutant77.miso`.

### Paso 4 — Comprobación
**Orquestador** (`pnpm tauri dev`; lo visual es del titular):
- Ctrl+S con un `.docx` de títulos y negritas: guarda `.txt` y `.rtf`; ambos reabren en Miso sin errores en el log. Cancelar no deja log de error.

**Titular:**
- `pnpm tauri build`; copiar `src-tauri/target/release/miso.exe` a otra carpeta y abrirlo: icono en la ventana y la barra de tareas; un PDF abre.
- CSP: `pnpm tauri build --debug`, abrir `src-tauri/target/debug/miso.exe`, F12: `fetch('https://example.com')` falla por CSP; la consola no muestra violaciones al abrir un documento ni al cambiar prefs, y las columnas y las prefs se aplican igual que en `tauri dev`.
- Ctrl+S: el `.rtf` exportado se abre en Word (o LibreOffice) con títulos y negritas, y sólo los títulos en negrita.

**Gate:** confirmación del titular.

### Paso 5 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `DIFERIDOS.md`, `REGISTRO.md`, este plan
- `ARCHITECTURE.md`: §4 (`export.rs`, `assets/miso.svg`; `pdfium/` se embebe en el `.exe`), §5 (`save_copy` sin entrada; `Write` hecho; el `Doc` se conserva), §9.1 y §9 PDF (sin asociaciones; DLL embebida y extraída a la carpeta temporal), §13 (fase 7 = exportar copia y ejecutable propio, hecha).
- `DIFERIDOS.md`: fuera CSP, `pdfium.dll` y dominio; nuevo (sin fase): instalador y asociaciones «Abrir con» si algún día se distribuye — sin `bundle.fileAssociations`, que en NSIS escribe el predeterminado de `HKCU\Software\Classes\.ext`; usar `installerHooks` con `OpenWithProgids` — con las licencias de pdfium y Literata (OFL).
- `REGISTRO.md`: asiento de fase 7. Este plan → `.historial/_PLAN_FASE_7_completado.md`.
- Commit `feat: fase 7 — exportar copia y ejecutable propio` (incluye `assets/miso.svg`).

**Gate:** `pnpm verify` verde sobre el commit final; `git status` limpio.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P4 ─→ P5
P1 ─→ P3 ─┘
```
P2 y P3 no comparten archivos.

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base | ✅ completado | `pnpm verify` = 0 (Rust 162, unit 98, layout 32) | |
| P2 — Exportar copia | ✅ completado | `pnpm verify` = 0 (Rust 168, unit 102, layout 32); negativa: sin `\plain` fallan 3 tests de `export` (repetida por el orquestador) | Agente Haiku; revisado |
| P3 — Id, icono y DLL | ✅ completado | `pnpm verify` = 0; test de extracción de la DLL | Agente Haiku; revisado |
| P4 — Comprobación | ✅ completado | Orquestador: `tauri dev` con `protegido.pdf` → DLL extraída a `%TEMP%` (7 586 816 bytes, igual que la embebida), aviso `Protected`, log en `%LOCALAPPDATA%\com.mutant77.miso\logs` | Titular: Ctrl+S (`.txt` y `.rtf`), `.exe` de *release* copiado (icono, PDF) y CSP, todo correcto. El primer `tauri build` falló por un bloqueo de archivo de Windows (error 32, ajeno al código); repetido, correcto |
| P5 — Cierre y commit | ✅ completado | `pnpm verify` = 0 | |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- Instalador, asociaciones de archivo, licencias empaquetadas, firma de código (C8; diferido).
- RTFD, exportar a otros formatos, fuentes/tamaños/colores en el RTF exportado.
- Migrar prefs y posiciones del id provisional.

**Hallazgos aparcados**
- (vacío)

**Desvíos de la implementación (2026-11-14)**
- Gate de P3: `%APPDATA%\com.mutant77.miso` no se crea al arrancar, sino al guardar prefs o posiciones por primera vez (comportamiento previo). El id nuevo se comprueba por el log en `%LOCALAPPDATA%\com.mutant77.miso\logs`.
- `tauri icon` genera además `64x64.png` (fuera de `bundle.icon`); se conserva como el resto de su salida.
- Ctrl+Mayús+S también guarda, como Ctrl+Mayús+O abre (el bloque Ctrl no mira Mayús).
- Un run sin negrita dentro de un título vuelve en negrita al reabrir el RTF: consecuencia aceptada de C5 (`\b` del título).
- Ctrl+S no lo prueba el orquestador: el diálogo nativo no se automatiza; la exportación está cubierta por los tests de `export.rs` (incluida la ida y vuelta).

## 10. Auditoría

2026-11-14, agy, OpenCode (Big Pickle) y pi (Nemotron 3 Ultra), sólo lectura y sin respaldo físico, sobre la versión con instalador NSIS; deliberación del orquestador (Opus). Después, el titular quitó el instalador (C8): lo auditado de NSIS pasa a `DIFERIDOS.md`.

- **Aplicado:** `\pard\plain` por bloque (la negrita de un título se filtraba a los siguientes; Big Pickle y orquestador); espacio tras la última palabra de control y `}` final (agy); escape de todo lo que no sea 0x20–0x7E; nombre propuesto sin extensión, comprobado en `rfd` (`SetDefaultExtension`) (agy, Big Pickle); `take` conserva la entrega única y `Slots::doc` sirve también el pendiente (agy, Big Pickle); `Write` con el nombre del archivo, no la ruta (agy); límite de la heurística de texto plano en C4 (agy, Big Pickle); prueba de CSP sin `eval` en consola y sin WordPad (Big Pickle); test de ida y vuelta con título seguido de párrafo.
- **Simplificado:** fuera la prueba negativa de P5: el test de ida y vuelta ya falla sin los escapes (Big Pickle).
- **Rechazado:** la cabecera RTF cierra el grupo raíz (agy): las llaves cuadran (`{\rtf1` sigue abierto). `style-src 'unsafe-inline'` de entrada (Big Pickle): Svelte aplica `style:` por CSSOM, comprobado en su código; sólo si P4 lo pide. `format_for` aparte (Big Pickle): es una comparación de extensión. Separador `\n` en el texto exportado y `\tab` que deja un espacio (Nemotron): `plain.rs` normaliza `\r\n` y el espacio delimitador nunca es texto en `rtf.rs`. `\outlinelevel` 6–9 y `\uc` en páginas de dos bytes (Nemotron): son del importador, fuera de esta fase. El resto de Nemotron repetía tareas del plan.
