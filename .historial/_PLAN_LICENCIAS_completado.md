# Plan de Implementación: avisos de licencia embebidos

## 1. Decisiones fijadas

> [!IMPORTANT]
> **Decisiones vinculantes e incuestionables.** Acordadas con el titular. En auditorías e implementación queda prohibido cuestionarlas, modificarlas o reabrirlas; la auditoría se limita a evaluar solidez técnica, casos borde y cumplimiento de los pasos dentro de este marco.

| ID | Decisión | Por qué | Ámbito |
| -- | -------- | ------- | ------ |
| D1 | Los textos se embeben **en Rust** con `include_str!` y se sirven por comando IPC; el frontend los recibe tipados con ts-rs | Coherencia con D10 («E/S sólo en Rust») y con `pdfium.dll`; no engorda el bundle JS | Rust, IPC |
| D2 | La entrada de la UI es una **sección «Acerca de» dentro del panel de Preferencias** (`Preferences.svelte`), con botón «Licencias» que sustituye el contenido del panel | Ya existe panel con scroll, foco y cierre; no añade ventanas ni atajos | Frontend |
| D3 | Los **11 catálogos** de `locales/` reciben los rótulos nuevos; los textos legales no se traducen | Regla vigente del proyecto: catálogos completos | i18n |
| D4 | Un **único generador** (`scripts/licenses.mjs`) produce los tres artefactos: tabla Rust, `THIRD_PARTY_NOTICES.md` y modo `--check` | Evita duplicar la lógica de atribución en dos lenguajes | Tooling |
| D5 | El manifiesto de licencias es **explícito**: un archivo sin entrada en el manifiesto hace fallar el generador | Es la puerta que detecta una actualización de PDFium con dependencias nuevas | Tooling |
| D6 | Los artefactos generados **se commitean** y se verifican con `--check` en `pnpm verify`; el build **no** los regenera | Un build que muta archivos versionados es silencioso y peligroso | Tooling, CI |
| D7 | Los textos completos van **dentro del `.md` generado**, no como referencias a archivos del repo | El release se descarga suelto; debe ser autosuficiente | Distribución |
| D8 | `tauri.conf.json` no se toca en este plan (`bundle.licenseFile` pertenece al trabajo de instalador) | El bundle está inactivo; configurarlo ahora no verifica nada | Fuera de alcance |

## 2. Objetivo

Que los avisos de licencia de PDFium, Literata y Miso **viajen dentro del `.exe`** y sean legibles por el usuario final, y que el fichero de avisos del release se genere desde las mismas fuentes sin intervención manual. Al terminar, ninguna obligación de atribución depende de la memoria de quien publica, y añadir o actualizar una dependencia con licencia propia rompe la verificación si no se declara.

## 3. Resumen del análisis

**Stack verificado.** Tauri 2 + Svelte 5 ($state, $props, $derived) + TS 6 + Vite 8; Rust edition 2024, `rust-version = 1.90`; ts-rs 12 exportando a `src/lib/ipc/types` vía `TS_RS_EXPORT_DIR` en `.cargo/config.toml:2`; Node 24 y pnpm 10 disponibles.

**Archivos leídos y su papel.**

| Archivo | Rol |
|---|---|
| `src-tauri/src/commands.rs` | Comandos con `#[tauri::command]`; ninguno devuelve datos estáticos todavía |
| `src-tauri/src/lib.rs` | Lista `generate_handler!` (líneas con `commands::`), mods del crate |
| `src-tauri/src/model.rs` | Tipos serializables con `#[derive(Serialize, TS)] #[ts(export)]` |
| `src/lib/ipc/commands.ts` | Wrappers `invoke<T>()` uno por comando |
| `src/lib/i18n.svelte.ts` | `Key = keyof typeof en`; `catalogs` importa los 11 JSON |
| `src/components/Preferences.svelte` | `<aside class="panel">` con `<form>`; `user-select: none` en el panel; carga asíncrona por llamada directa (`listFonts().then`) |
| `src/components/Reader.svelte` | Abre/cierra `Preferences`; `Escape` cierra el panel (línea 82) |
| `package.json` | `verify` = `lint:rust && test:rust && check && test && test:layout` |
| `src-tauri/pdfium/licenses/*` | 18 textos; más `src-tauri/pdfium/LICENSE` (pdfium-binaries), `src/assets/fonts/OFL.txt` y `LICENSE` raíz: 21 fuentes |
| `src-tauri/src/i18n.rs` | Incluye los 11 catálogos; test `catalogos_con_las_mismas_claves_y_variables_que_el_ingles` |
| `THIRD_PARTY_NOTICES.md` | Hoy manual: versión de PDFium (`158.0.8125.0`, release `chromium/8125`) y enlaces de origen |

**Patrones existentes.** Datos servidos por Rust y consumidos con `invoke<T>()`; tipos TS generados en `cargo test` (no se editan a mano); tests unitarios junto al módulo (`*.test.ts`), sin tests de componentes; textos de interfaz sólo por `t()`.

**Cobertura de pruebas.** No hay tests de componentes. El área afectada está cubierta por `pnpm check` (tipos), `lint:rust`, `test:rust` y `test` (i18n incluido, `src/lib/i18n.test.ts`). La completitud de catálogos (mismas claves y variables que `en`, ninguna vacía) ya la garantiza el test de Rust en `i18n.rs`.

**Riesgos detectados.** (a) `pnpm licenses` **ya es un comando de pnpm**: el script no puede llamarse `licenses`. (b) El panel fija `user-select: none`, que impediría copiar el texto legal. (c) `Escape` en `Reader` cierra todo el panel, así que desde la vista de licencias también cerrará: comportamiento aceptado, la vuelta es por botón. (d) `include_str!` con rutas relativas desde `src-tauri/src/` depende de que el generador escriba esas rutas exactas; un fallo ahí rompe la compilación, no la ejecución. (e) El índice de git guarda los textos en LF, pero la copia de trabajo actual tiene 11 de `pdfium/licenses/` en CRLF y `pdfium/LICENSE` mixto (extraídos antes de `.gitattributes`): la salida del generador y el texto embebido dependerían de la máquina. `ARCHITECTURE.md` §5 fija que el frontend nunca recibe texto sin normalizar. (g) Rust interpreta `\` en literales como escape: una ruta de Windows en `include_str!` no compila. (f) `t()` está tipado con `Key`: usar una clave antes de que exista en `en.json` rompe `pnpm check`.

## 4. Restricciones innegociables

- Sin dependencias nuevas (ni npm ni crates).
- Los textos legales no se editan: se reproducen íntegros desde sus archivos de origen; las únicas transformaciones admitidas son normalizar el fin de línea a LF y, en el origen, transcodificar a UTF-8 un archivo que no lo sea (mismos caracteres; `include_str!` exige UTF-8). El generador falla ante cualquier fuente no UTF-8.
- Los archivos generados se escriben en UTF-8 **sin BOM** y con **LF**.
- `src/lib/ipc/types/*` no se edita a mano; sale de `cargo test`.
- Nada de `pnpm build` ni compilaciones de release: las ejecuta el titular.
- Los comandos nuevos no requieren permisos en `capabilities/reader.json` (comandos propios, no de plugin; `build.rs` no declara manifiesto de app).

* **Descartados:**
  - **Desechada «copiar byte a byte»:** con copias de trabajo en CRLF contradice la regla de LF y hace que `--check` dependa de la máquina.
  - **Desechado leer las fuentes con decodificación UTF-8 permisiva:** `freetype.txt` venía en Windows-1252 (`©` = `0xA9`); el `.md` salía con U+FFFD y `include_str!` no compilaba. Se transcodificó el archivo y el generador decodifica en modo estricto.
  - **Desechado `include_bytes!` + `encoding_rs` para conservar la codificación original:** complica Rust por un único byte; el texto es el mismo en UTF-8.

## 5. Checklist previo a los pasos

- [x] Estructura y stack revisados.
- [x] Archivos afectados leídos (ninguno supuesto).
- [x] Patrones identificados: comando Rust + wrapper `invoke<T>()` + tipo ts-rs.
- [x] Decisiones D1–D8 fijadas con el titular.
- [x] Cobertura de pruebas del área mapeada.
- [x] Versiones verificadas (ts-rs 12, Tauri 2, Svelte 5, Node 24).
- [x] Consumidores de `Preferences.svelte` y `locales/*` localizados.

## 6. Pasos secuenciales

### Paso 1 — Línea base

**Archivos afectados:** ninguno.

Ejecutar `pnpm verify` y anotar en la tabla de seguimiento el commit y el resultado observado (números de tests de Rust, unit y layout). Sirve de comparación al final: el recuento de tests sólo puede crecer.

**Puerta:** `pnpm verify` en verde y hash de commit registrado en la tabla.

### Paso 2 — Generador y manifiesto

**Archivos afectados:** `scripts/licenses.mjs` (nuevo), `package.json`, `THIRD_PARTY_NOTICES.md` (pasa a generado).

El script, sin dependencias, contiene el **manifiesto** como array ordenado de entradas `{ title, kind, origin, path }`. `kind` es un identificador SPDX o un rótulo neutro en inglés (se muestra sin traducir en la UI). `origin` es la URL del proyecto o `Bundled in PDFium`.

| title | kind | origin | path |
|---|---|---|---|
| Miso | MIT | este repositorio | `LICENSE` |
| Literata | OFL-1.1 | https://github.com/googlefonts/literata | `src/assets/fonts/OFL.txt` |
| pdfium-binaries | MIT | https://github.com/bblanchon/pdfium-binaries | `src-tauri/pdfium/LICENSE` |
| PDFium | BSD-3-Clause | https://pdfium.googlesource.com/pdfium/ | `src-tauri/pdfium/licenses/pdfium.txt` |
| Abseil | Apache-2.0 | Bundled in PDFium | `.../abseil.txt` |
| Anti-Grain Geometry 2.3 | Permissive notice | Bundled in PDFium | `.../agg23.txt` |
| Dragonbox (LLVM) | Apache-2.0 WITH LLVM-exception | Bundled in PDFium | `.../dragonbox-Apache2-LLVM.txt` |
| Dragonbox (Boost) | BSL-1.0 | Bundled in PDFium | `.../dragonbox-Boost.txt` |
| fast_float | MIT | Bundled in PDFium | `.../fast_float.txt` |
| FreeType | FTL | Bundled in PDFium | `.../freetype.txt` |
| HarfBuzz | MIT-Modern-Variant | Bundled in PDFium | `.../harfbuzz.txt` |
| Highway | Apache-2.0 | Bundled in PDFium | `.../highway.txt` |
| ICU | Unicode-3.0 | Bundled in PDFium | `.../icu.txt` |
| Little CMS | MIT | Bundled in PDFium | `.../lcms.txt` |
| libjpeg-turbo (IJG) | IJG | Bundled in PDFium | `.../libjpeg_turbo.ijg` |
| libjpeg-turbo (BSD) | BSD-3-Clause AND Zlib | Bundled in PDFium | `.../libjpeg_turbo.md` |
| OpenJPEG | BSD-2-Clause | Bundled in PDFium | `.../libopenjpeg.txt` |
| libpng | libpng-2.0 | Bundled in PDFium | `.../libpng.txt` |
| LLVM libc | Apache-2.0 WITH LLVM-exception | Bundled in PDFium | `.../llvm-libc.txt` |
| simdutf | MIT | Bundled in PDFium | `.../simdutf.txt` |
| zlib | Zlib | Bundled in PDFium | `.../zlib.txt` |

Comportamiento:

1. Verifica que cada `path` del manifiesto existe y que **no hay archivos en `src-tauri/pdfium/licenses/` sin entrada**. Cualquier desajuste: salida con la lista y código 1.
2. Lee cada texto y normaliza `\r\n` → `\n`. Si algún texto contiene ` ``` `, falla con su nombre (rompería el bloque del paso 4).
3. Genera `src-tauri/src/licenses_data.rs` con `pub const ENTRIES: &[(&str, &str, &str)]` (título, tipo, cuerpo) usando `include_str!` con rutas relativas a `src-tauri/src/` y siempre con `/` (nunca `path.relative` sin convertir `\`): `../pdfium/licenses/x.txt`, `../../LICENSE`, `../../src/assets/fonts/OFL.txt`. Título y tipo se emiten como literales escapados con `JSON.stringify`.
4. Genera `THIRD_PARTY_NOTICES.md`: intro fija (Miso es MIT; incluye `pdfium.dll` x64 versión 158.0.8125.0, release `chromium/8125` de pdfium-binaries; componentes de terceros con licencias permisivas), tabla `Componente | Licencia | Origen`, y los textos completos bajo `## <título>`, cada uno dentro de un bloque ` ``` `, en el orden del manifiesto. La versión de PDFium vive como constante en el script: actualizar PDFium obliga a tocarla.
5. Modo `--check`: regenera en memoria y compara con el disco; si difiere, escribe el listado de archivos desactualizados y sale con código 1 sin tocar nada.

`package.json`: `"gen:licenses": "node scripts/licenses.mjs"` y `"check:licenses": "node scripts/licenses.mjs --check"` (nunca `licenses`, que ya es comando de pnpm).

**Puerta:** `pnpm gen:licenses` deja los dos archivos; una segunda ejecución no cambia ningún byte respecto a la primera; `pnpm check:licenses` sale 0; añadir a mano una línea a `src-tauri/pdfium/licenses/zlib.txt` hace que `pnpm check:licenses` falle señalando `THIRD_PARTY_NOTICES.md` como desactualizado, y revertirla lo devuelve a 0; crear un `.txt` nuevo en `licenses/` lo hace fallar nombrando el huérfano; convertir `zlib.txt` a CRLF **no** hace fallar `check:licenses`. `THIRD_PARTY_NOTICES.md` contiene la tabla de 21 filas con origen, la versión de PDFium y el texto íntegro de la OFL; ningún archivo generado contiene `\r`.

* **Descartados:**
  - **Desechado el manifiesto `{ title, kind, path }`:** no tenía de dónde sacar la columna «Origen» y el `.md` generado perdía la versión de PDFium y los enlaces que hoy constan.
  - **Desechados los tipos en español («Permisiva (aviso)», «MIT antigua»):** no son texto legal y se mostrarían sin traducir en 11 idiomas; se usan SPDX o rótulos neutros en inglés.
  - **Desechado pegar los textos como Markdown suelto:** `*`, `_`, `<` y sangrías se interpretan como formato; `libjpeg_turbo.md` es Markdown dentro de Markdown.
  - **Desechada la puerta «`git status` limpio tras el primer commit»:** no hay commit hasta el final; la idempotencia se comprueba entre dos ejecuciones.
  - **Desechado el paso previo `git checkout --` sobre `src-tauri/pdfium`:** con el árbol limpio para git es un no-op y no reescribe los CRLF; la normalización la hacen el generador (para el `.md`) y Rust (para el IPC).
  - **Desechada la puerta «falla con el nombre de `zlib.txt`»:** `--check` compara artefactos generados; lo que lista es `THIRD_PARTY_NOTICES.md`.

### Paso 3 — Tipo y comando en Rust

**Archivos afectados:** `src-tauri/src/model.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/ipc/types/License.ts` (generado).

- `model.rs`: `pub struct License { pub title: String, pub kind: String, pub body: String }` con `Serialize, Clone, Debug, TS` y `#[ts(export)]`.
- `lib.rs`: `mod licenses_data;` y `commands::get_licenses` en `generate_handler!`.
- `commands.rs`: `get_licenses() -> Vec<License>` que mapea `crate::licenses_data::ENTRIES`, con `body: body.replace("\r\n", "\n")` (`ARCHITECTURE.md` §5).
- Test en `commands.rs`: la tabla no está vacía, ningún título ni cuerpo vacíos, ningún cuerpo contiene `\r`, y existe una entrada `Miso` y otra `PDFium`.

**Puerta:** `pnpm lint:rust` y `pnpm test:rust` en verde; `src/lib/ipc/types/License.ts` existe y declara `title`, `kind`, `body` como `string`; `pnpm check` sin errores.

* **Descartados:**
  - **Desechado `mod licenses_data;` dentro de `licenses.rs`:** desde un archivo no `mod.rs`, Rust busca `src/licenses/licenses_data.rs` y no compila.
  - **Desechado el módulo `licenses.rs`:** sólo envolvía un mapeo de cuatro líneas; cabe en el propio comando.
  - **Desechado `get_version`:** no forma parte del objetivo (licencias); añadía comando, wrapper, clave en 11 idiomas y un hallazgo aparcado sobre la versión duplicada.

### Paso 4 — Rótulos en los 11 catálogos

**Archivos afectados:** `locales/{en,es,pt,de,it,fr,nl,ru,nb,sv,fi}.json`.

Claves nuevas, presentes en los 11: `prefs.about`, `prefs.licenses`, `licenses.title`, `licenses.back`. `en.json` es el origen de `Key`; el resto se traduce con el mismo criterio que las claves existentes.

**Puerta:** `pnpm check`, `pnpm test` y `pnpm test:rust` en verde (el test de catálogos de `i18n.rs` exige las 4 claves en los 11).

* **Descartados:**
  - **Desechado hacer los rótulos después de la UI:** `t()` está tipado con `Key`; la UI usaría claves inexistentes y `pnpm check` fallaría en su propia puerta.
  - **Desechado el test que comprueba sólo las claves nuevas:** redundante.
  - **Desechado un test de completitud en `i18n.test.ts`:** ya existe en Rust (`i18n.rs`) y cubre además las variables; duplicarlo obligaría a exportar `catalogs`.
  - **Desechadas `prefs.version` y `licenses.kind`:** la primera iba con `get_version`; la segunda no tenía uso definido (el tipo SPDX se muestra tal cual).

### Paso 5 — Vista de licencias e integración en Preferencias

**Archivos afectados:** `src/lib/ipc/commands.ts`, `src/components/Licenses.svelte` (nuevo), `src/components/Preferences.svelte`.

- `commands.ts`: `getLicenses(): Promise<License[]>`.
- `Licenses.svelte`: sin props; sólo el cuerpo. Carga con llamada directa `getLicenses().then(...)` al crear el componente (mismo patrón que `listFonts` en `Preferences`); lista de `<article>` con `<h2>{title}</h2>`, tipo de licencia en texto pequeño y `<pre>{body}</pre>` con `white-space: pre-wrap`; `user-select: text` en el contenedor (el panel lo desactiva), para que títulos, tipo y cuerpo se copien juntos.
- `Preferences.svelte`: `let view = $state<'prefs' | 'licenses'>('prefs')`. La cabecera existente es la única: con `prefs`, título «Preferencias» y ✕; con `licenses`, botón «Volver» (recibe el foco al aparecer), título «Licencias» y ✕ en su sitio. Debajo, `<form>` o `<Licenses />` según `view`. Sección nueva al final del formulario: «Acerca de» con botón «Licencias» a ancho completo (`grid-column: 1 / -1`). Al volver, el foco va al primer control del formulario (el `$effect` existente): comportamiento aceptado.

**Puerta:** `pnpm check` y `pnpm test` en verde; en `pnpm tauri dev`, Preferencias muestra el botón, el botón abre la vista con una sola cabecera y el foco en «Volver», el texto se puede seleccionar y copiar, «Volver» regresa al formulario y `Escape` cierra el panel.

* **Descartados:**
  - **Desechada la carga con `$state` + `$effect`:** el componente ya usa llamada directa para datos asíncronos; el `$effect` existente sólo gestiona el foco.
  - **Desechado no gestionar el foco al cambiar de vista:** el botón «Licencias» desaparece y el foco quedaba en `body`.
  - **Desechada la cabecera propia en `Licenses.svelte` (props `onback`):** la cabecera de `Preferences` está fuera del `<form>`; habría dos cabeceras apiladas.
  - **Desechado `user-select: text` sólo en `<pre>`:** títulos y tipo quedaban sin poder copiarse.

### Paso 6 — Puerta de verificación y documentación

**Archivos afectados:** `package.json`, `ARCHITECTURE.md`, `README.md`, `REGISTRO.md`.

- `verify` pasa a `pnpm check:licenses && pnpm lint:rust && pnpm test:rust && pnpm check && pnpm test && pnpm test:layout`.
- `ARCHITECTURE.md`: árbol de §4 (`scripts/licenses.mjs`, `src/components/Licenses.svelte`, `licenses_data.rs` marcado como generado), tabla de comandos de §5 (`get_licenses`), y §12 con la puerta `check:licenses`.
- `README.md`: la sección *License* menciona que los avisos se leen en Preferencias → Licencias y que `THIRD_PARTY_NOTICES.md` es generado.
- `REGISTRO.md`: entrada del día.

**Puerta:** `pnpm verify` completo en verde con `check:licenses` como primera etapa; romper a propósito `licenses_data.rs` (borrar una línea) lo detecta `pnpm check:licenses`, y `pnpm lint:rust` falla si el archivo generado no compila.

### Paso 7 — Verificación final

**Archivos afectados:** ninguno de código.

`pnpm verify` y recorrido manual en `pnpm tauri dev`: Preferencias → Licencias → comprobar que aparecen las 21 entradas, que el texto de PDFium y el de la OFL están completos (comparar longitud con los archivos de origen) y que copiar y pegar en un editor conserva el texto. Comparar el recuento de tests con la línea base del Paso 1.

**Puerta:** `pnpm verify` en verde y los tres puntos del recorrido observados. Commit final.

## 7. Grafo de dependencias

```
P1 línea base
      │
      ├─→ P2 generador ──→ P3 Rust (necesita licenses_data.rs) ─┐
      │                                                        ├─→ P5 UI ──→ P6 verify+docs ──→ P7 final
      └─→ P4 i18n ─────────────────────────────────────────────┘
```

P2→P3 y P4 son independientes entre sí; P5 requiere el tipo generado en P3 y las claves de P4.

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
| ---- | ------ | ------------ | ----- |
| P1 — Línea base | ✅ hecho | `8d47515`; verify verde: Rust 226, unit 114, layout 77 | |
| P2 — Generador y manifiesto | ✅ hecho | idempotente; check 0; negativas (línea extra, huérfano, CRLF, no UTF-8) | `freetype.txt` transcodificado de Windows-1252 a UTF-8 |
| P3 — Tipo y comando | ✅ hecho | clippy limpio; Rust 228; `License.ts` generado | |
| P4 — Rótulos i18n | ✅ hecho | test de catálogos de `i18n.rs` verde; check 0/0 | |
| P5 — Vista e integración | ✅ hecho | check 0/0; unit 114 | recorrido visual pendiente del titular |
| P6 — Puerta y documentación | ✅ hecho | verify verde con `check:licenses` primero; `.rs` roto detectado | |
| P7 — Verificación final | ◐ parcial | verify verde: Rust 228, unit 114, layout 77 (≥ base) | recorrido manual en `tauri dev` → `DIFERIDOS.md` |

## 9. Fuera de alcance

**Exclusiones deliberadas**

- Instalador, `bundle.licenseFile`, asociaciones de archivo y firma de código.
- Workflow de release en GitHub Actions y automatización del `push` de tags: no existe `.github/` y depende de decisiones pendientes (instalador, firma, updater).
- Licencias de las dependencias transitivas de Rust y npm (`cargo-about`, `license-checker`).
- Tests de componentes Svelte: el proyecto no tiene ninguno y este plan no introduce el patrón.
- Mostrar la versión de la app en «Acerca de».

**Hallazgos aparcados**

- La versión vive duplicada en `tauri.conf.json` y `Cargo.toml`; este plan ya no la lee, pero el duplicado sigue existiendo.
- `Escape` cierra el panel completo desde la vista de licencias; una pila de `Escape` por vista sería más fino y no es necesaria hoy.
- `THIRD_PARTY_NOTICES.md` deja de ser editable a mano: pasa a generado. Quien lo edite verá el cambio revertido por `gen:licenses`.
- El control de huérfanos sólo vigila `src-tauri/pdfium/licenses/`; una fuente nueva en `src/assets/fonts/` con licencia propia no lo dispararía.
