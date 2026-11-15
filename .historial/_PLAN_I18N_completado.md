# Plan de Implementación: Interfaz multilingüe

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes:** durante auditoría o implementación **queda prohibido reabrirlas**. No se reabren `DECISIONES.md` ni D1–D11 de `ARCHITECTURE.md`.
> I1 **reabre ARCHITECTURE §5** («UI sólo en español, sin i18n»): petición del titular, 2026-11-15. I7, I8 e I11 aprobadas por el titular el 2026-11-15.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| I1 | **11 idiomas de interfaz:** `en` (defecto y respaldo), `es`, `pt`, `de`, `it`, `fr`, `nl`, `ru`, `nb`, `sv`, `fi`. | Petición del titular. | todo |
| I2 | **Catálogo único compartido:** `locales/<código>.json`, plano (`"clave": "texto"`), uno por idioma. Rust los incrusta con `include_str!`; el frontend los importa con Vite. Sin bibliotecas (ni i18next, ni Fluent, ni ICU). | Un solo origen de verdad para Rust y Svelte; ~30 cadenas no justifican una biblioteca. | P2 |
| I3 | **Sin plurales.** Las frases se redactan para no necesitarlos («{current} de {total}», «Y {n} más.» → ru «И ещё {n}.»). Variables `{nombre}` sustituidas con `replace`. | Evita reglas plurales (ruso, finés); ninguna frase actual las exige. | P2 |
| I4 | **Preferencia `language` en `Prefs`:** `"auto"` (defecto) o un código de I1. Validación por recorte: valor desconocido → `"auto"`. `#[serde(default)]` ya existente: un `prefs.json` antiguo carga con `"auto"`. | Reutiliza persistencia, validación y `prefs-changed`: el cambio manual llega en vivo a todas las ventanas sin evento nuevo. | P2 |
| I5 | **Detección del sistema en Rust**, una vez al arrancar, con `sys-locale::get_locales()`: primer idioma de la lista de preferencias del usuario que esté en I1 (por subetiqueta primaria; `no`/`nn`/`nb` → `nb`); si ninguno, `en`. | La lista completa acierta con sistemas en un idioma no soportado y un segundo idioma soportado (p. ej. catalán + español → `es`). Multiplataforma, sin tocar `windows`. | P2 |
| I6 | **Resolución única:** idioma efectivo = `language` si no es `"auto"`, si no el detectado (I5). Rust lo inyecta en `create_reader` junto a `__PREFS__` como `window.__SYSTEM_LANG__`; el frontend aplica la misma regla de una línea. | Rust y Svelte siempre dan el mismo idioma; sin IPC nuevo y sin parpadeo en el primer fotograma. | P2 |
| I7 | **`Restablecer` no toca `language`** (`reset_prefs` copia el `language` vigente sobre `Prefs::default()`). | El botón está en la sección de tipografía y colores; perder el idioma elegido sorprendería. Coste: una línea. | P2 |
| I8 | **Portugués de Brasil** para `pt` (sin marcas regionales en el texto que choquen en Portugal); noruego **bokmål** para `nb`. Ningún idioma se representa con banderas: sólo endónimos. | Variante única por idioma; bokmål es la norma escrita mayoritaria. | P3 |
| I9 | **Atajos dentro de la cadena traducida** («Abre un archivo con Ctrl+O», «Cerrar (Esc)»). En `de`: `Strg`. | Los teclados alemanes rotulan `Strg`; el traductor decide la forma. | P3 |
| I10 | **Fuera de alcance:** botones de los diálogos nativos (Abrir/Cancelar/Aceptar los pone Windows en su idioma), mensaje de E/S del sistema (`os_message`, idioma de Windows), log (sigue en español), partición de palabras y `lang` de `.columns` (dependen del idioma del **documento**, no de la interfaz; ya en `DIFERIDOS.md`). El título «Portapapeles» de un documento se fija al crearlo y no cambia si luego se cambia el idioma. | Son del sistema o del documento, no de la interfaz. | — |
| I11 | **Literata cirílico:** añadir los subconjuntos `cyrillic` y `cyrillic-ext` (woff2 de `@fontsource-variable/literata` 5.3.0, la misma versión que los latinos, OFL) a `src/fonts.css`, con sus `unicode-range`. | La interfaz rusa usa Segoe UI (ya tiene cirílico), pero un libro en ruso caería hoy a otra fuente. | P4 |

## 2. Objetivo

Miso arranca en el idioma del sistema (inglés si no está entre los 11) y el usuario puede fijar otro desde Preferencias; el cambio se ve al instante en todas las ventanas y en los mensajes de error siguientes.

## 3. Resumen del análisis

- **Inventario de textos visibles (32):**
  - Rust: `error.rs::user_message` (7 variantes) y `batch_message` («Y {n} más.»); `commands.rs` filtros «Documentos», «Todos», «Texto» («RTF» no se traduce); `clipboard.rs` título «Portapapeles». El título «Miso» de ventana y de diálogo no se traduce.
  - Svelte: `Preferences.svelte` (título y `aria-label` del panel, Cerrar, 3 secciones, 8 etiquetas, Tema/Claro/Oscuro, «Ctrl+I alterna», Restablecer), `FindBar.svelte` (Buscar ×2, Sin resultados, «{current} de {total}»), `EmptyState.svelte` (1). `index.html` fija `lang="es"`.
  - El selector de fuentes del sistema (`ae967d5`) sólo añade nombres de fuente: nada que traducir.
- **Puntos de enganche existentes:** `Prefs` + `prefs::apply` + `prefs-changed` (cambio en vivo); `create_reader` ya inyecta `__PREFS__` con `initialization_script`.
- **Estado global en Rust:** `user_message()` no recibe `AppHandle`. Un `AtomicU8` guarda el índice del idioma efectivo (por defecto 0 = `en`). Cero cambios de firma en `report`.
- **Riesgos:**
  1. Calidad de traducción: las hace el agente; el titular revisa al menos `es` y `en`. Textos breves y sin jerga.
  2. Anchura: alemán y finés alargan etiquetas; la rejilla del panel (`1fr 128px`) ya recorta con `minmax(0, 1fr)`. Revisión visual del titular.
  3. Tests de Rust en paralelo sobre un global: ningún test llama a `i18n::set`, así que el global queda en `en` durante `cargo test`; los mensajes en otro idioma se comprueban con las variantes `_in(code)`.

## 4. Restricciones innegociables

- Nunca `pnpm build` ni `pnpm tauri build`. Puertas desde PowerShell.
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo (los JSON llevan `«»`, `ä`, cirílico).
- D10/D11 intactos: el frontend no muestra errores; los textos de error siguen viviendo sólo en Rust (ahora con clave de catálogo).

## 5. Checklist previo a los pasos

- [x] Archivos afectados leídos (`error.rs`, `commands.rs`, `clipboard.rs`, `prefs.rs`, `lib.rs`, `doc_windows.rs::create_reader`, los tres componentes, `index.html`, `fonts.css`, `main.ts`, `tsconfig.json`, `vitest.config.ts`, `tests/layout/defaults.ts`)
- [x] Visto bueno del titular a I7, I8, I11 (2026-11-15)
- [x] Auditoría externa (DeepSeek 4.1 Flash vía OpenCode, 2026-11-15) incorporada

## 6. Pasos secuenciales

### Paso 1 — Línea base
- `git status` limpio salvo este plan (y `miso.exe` sin seguimiento); `pnpm verify` en verde. Punto de vuelta atrás: `ae967d5` (incluye el selector de fuentes del sistema).

**Gate:** `pnpm verify` = 0.

### Paso 2 — Infraestructura, español/inglés y sus tests
**Archivos:** `locales/en.json`, `locales/es.json` (nuevos), `src-tauri/src/i18n.rs` (nuevo), `src-tauri/Cargo.toml` y `Cargo.lock` (`sys-locale`), `prefs.rs`, `error.rs`, `commands.rs`, `clipboard.rs`, `lib.rs`, `doc_windows.rs`, `src/lib/i18n.svelte.ts` (nuevo), `src/lib/i18n.test.ts` (nuevo), `Reader.svelte`, `Preferences.svelte`, `FindBar.svelte`, `EmptyState.svelte`, `index.html`, `tsconfig.json`, `vitest.config.ts`, `tests/layout/defaults.ts`.
- **`i18n.rs`:** `LANGS: [&str; 11]` (`en` primero); catálogos con `include_str!("../../locales/<código>.json")`, parseados una vez (`OnceLock`); `detect(locales: &[String]) -> &'static str` (pura, I5); `system() -> &'static str` (el detectado, cacheado en `OnceLock`); `effective(language: &str) -> &'static str` (I6); `set(code)` y `current()`; `t(clave)` y `t_in(code, clave)` (clave ausente → inglés → la propia clave); `format(plantilla, &[(nombre, valor)])`.
- **Arranque (`lib.rs::setup`):** `PrefsState::load` → `i18n::set(i18n::effective(&prefs.language))` → `ventana_inicial`. Así el título del portapapeles y cualquier error temprano salen en el idioma correcto.
- **`prefs.rs`:** campo `language` (I4); `validated` lo recorta; `apply` llama a `i18n::set` **antes** de `report(Persist)`. Se regenera `Prefs.ts` (`cargo test`).
- **`commands.rs`:** filtros → `i18n::t`; `reset_prefs` recibe `State<PrefsState>` y aplica `Prefs { language: <vigente>, ..Prefs::default() }` (I7).
- **`error.rs`:** `user_message()` = `message_in(i18n::current())`; `message_in(code)` y `batch_message` construyen con `t_in`/`format`. **`clipboard.rs`:** título → `i18n::t`.
- **`doc_windows.rs::create_reader`:** añade `window.__SYSTEM_LANG__ = "<i18n::system()>"` al script que ya inyecta `__PREFS__`.
- **`i18n.svelte.ts`:** importa los catálogos (`../../locales/<código>.json`); `export const i18n = $state({ lang: 'en' })` (no un `let` exportado: Svelte prohíbe reasignar estado exportado); `t(clave, vars?)`, `effective(prefs)` y `setLang(código)`, que también fija `document.documentElement.lang`. `Reader.svelte` llama a `setLang(effective(prefs))` con `__PREFS__`, tras `getPrefs` y en `prefs-changed`.
- **`index.html`:** `lang="en"` (lo sobrescribe `setLang`). **`tsconfig.json`:** `"resolveJsonModule": true`. **`vitest.config.ts`:** `plugins: [svelte()]` también en el proyecto `unit` (para importar `.svelte.ts`). **`tests/layout/defaults.ts`:** `language: 'auto'`.
- **Componentes:** literales → `t(...)`, incluidos `aria-label` y `title`.
- **`Preferences.svelte`:** nueva sección «Idioma» con un `<select>` a lo ancho de las dos columnas: «Automático» (traducido) + los 11 por su **endónimo** (English, Español, Português, Deutsch, Italiano, Français, Nederlands, Русский, Norsk bokmål, Svenska, Suomi), sin traducir.
- **Tests:**
  - `i18n.rs`: `detect` con `["es-ES"]` → `es`, `["ca-ES","es-ES"]` → `es`, `["no-NO"]`/`["nn"]` → `nb`, `["ja-JP"]` → `en`, `[]` → `en`; `effective` con `"auto"` y con código; paridad de catálogos (cada uno con exactamente las claves de `en` y las mismas `{variables}` en cada frase).
  - `prefs.rs`: `language` inválido → `"auto"`; JSON sin el campo → `"auto"`.
  - `error.rs`: los tests existentes pasan a esperar inglés (global por defecto); uno nuevo comprueba `message_in("es")`. `clipboard.rs:50` compara con `i18n::t(...)`, no con un literal. Ningún test llama a `set` (§3, riesgo 3).
  - `i18n.test.ts`: sustitución de variables, clave ausente → inglés, `effective`.

**Gate:** `pnpm verify` = 0.

### Paso 3 — Resto de idiomas
**Archivos:** `locales/{pt,de,it,fr,nl,ru,nb,sv,fi}.json`; `i18n.rs` e `i18n.svelte.ts` los incluyen.
- Mismas claves y mismas variables que `en.json`. I3, I8, I9.

**Gate:** `pnpm verify` = 0 (la paridad cubre ya los 11).

### Paso 4 — Literata cirílico
**Archivos:** `src/assets/fonts/literata-cyrillic{,-ext}-opsz-normal.woff2`, `src/fonts.css`, `src/main.ts`, `tests/layout/defaults.ts`.
- Origen: `npm pack @fontsource-variable/literata@5.3.0` en el scratchpad (sin añadir dependencia); se copian los dos woff2 y sus `unicode-range` de su CSS.
- La muestra de `document.fonts.load` pasa de `'aáñ'` a `'aáñЖ'` en `main.ts` y `defaults.ts`, para medir columnas con el cirílico ya cargado.

**Gate:** `pnpm verify` = 0.

### Paso 5 — Cierre
- `ARCHITECTURE.md`: §5 (sustituir «UI sólo en español, sin i18n»), §4 (`locales/`, `i18n.rs`, `i18n.svelte.ts`), §8 (campo `language`; Literata incluye cirílico).
- `DECISIONES.md`: una línea con I1, I5 y la variante de I8.
- `REGISTRO.md` + commit. Plan → `.historial/_PLAN_I18N_completado.md`. Diferidos, si quedan, a `DIFERIDOS.md`.

## 7. Verificación del titular (manual)

- Windows en español → Miso en español; quitar español de las preferencias de idioma → inglés.
- Cambiar a Русский en Preferencias con dos ventanas abiertas: ambas cambian; abrir un `.bin` → error en ruso.
- Revisar en `de` y `fi` que las etiquetas del panel caben.
