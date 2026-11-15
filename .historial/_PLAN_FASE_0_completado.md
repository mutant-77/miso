# Plan de Implementación: Fase 0 — Scaffold Tauri 2 + Svelte 5 + TS

## 1. Decisiones fijadas
> [!IMPORTANT]
> **Decisiones vinculantes e incuestionables:** fijadas con el titular. Durante auditoría o implementación **queda prohibido cuestionarlas, modificarlas o reabrirlas**. La auditoría se limita a la robustez técnica y a la ejecución de los pasos dentro de este marco.

| ID | Decisión | Por qué | Ámbito |
|---|---|---|---|
| F1 | `git init` al empezar; commit de línea base con la documentación y commit final. `Tofu.dmg` y `rea-tofu-evidence.*` quedan **fuera** de git (`.gitignore`). | Punto de vuelta atrás; AGENTS.md exige commit tras cada implementación. | P1, P5 |
| F2 | `test:layout` usa el **Edge instalado** (`channel: 'msedge'`) vía Playwright; no se descarga Chromium. | Mismo motor que WebView2. Ya reflejado en ARCHITECTURE §12. | P4 |
| F3 | Scaffold **manual** del frontend (sin `create-vite` ni `create-tauri-app`) + `tauri init` para `src-tauri/`. | `create-tauri-app --template svelte` trae SvelteKit (contra D5) y ambos traen demo que habría que borrar. | P2, P3 |
| F4 | Versiones: **Tauri 2.x** (`tauri = "2"`, no la 3 alpha), **TypeScript 6.x** (`svelte-check` 4.7 no admite TS 7), Svelte 5, Vite 8, `@sveltejs/vite-plugin-svelte` 7, Vitest 5. | Compatibilidad verificada de peer dependencies (ver §3). | P2–P4 |
| F5 | Identificador provisional **`com.provisional.miso`** hasta fijar el dominio (diferido de fase 7). | Tauri exige `identifier`; sigue el patrón `com.<dominio>.miso` y es obviamente provisional. Cambiarlo antes de fase 7 no tiene datos de usuario que migrar. | P3 |
| F6 | Ventana inicial con label **`reader-0`**; capability `reader` aplicada a `reader-*`. | Preparada para N ventanas de lectura (D4) y para que `App.svelte` decida por label. | P3 |
| F7 | Fase 0 **sin plugins** (ni dialog, ni log, ni single-instance) y sin `@tauri-apps/api` en el frontend. Se elimina lo que `tauri init` añada por defecto (p. ej. `tauri-plugin-log`). | Entran con su fase (log/dialog en 1, single-instance en 3). Evita código sin uso. | P3 |

## 2. Objetivo

Dejar montado el esqueleto del proyecto según ARCHITECTURE §4 y §13 (fase 0): la app abre **una ventana vacía** titulada "Miso", y existen las puertas locales `check`, `test`, `test:layout` y `clippy` agrupadas en `pnpm verify`. Nada de lógica de producto.

## 3. Resumen del análisis

- **Estado:** el directorio sólo contiene documentación (`AGENTS.md`, `ARCHITECTURE.md`, `DECISIONES.md`, `DIFERIDOS.md`, `REGISTRO.md`, `.historial/`), `Tofu.dmg` y `rea-tofu-evidence.{json,md}`. No hay código ni git.
- **Herramientas (2026-11-10):** Node 24.18.0, pnpm 10.33.2, rustc/cargo 1.98.1 (`stable-x86_64-pc-windows-msvc`), clippy 0.1.98, Visual Studio 2019/2022 instalados, WebView2 Runtime 157 (≥ 128 exigido en §10), git 2.45.2. `cargo tauri` no está instalado: se usa `@tauri-apps/cli` como dependencia de desarrollo.
- **Versiones npm:** `@tauri-apps/cli` 2.12.4, `svelte` 5.57.3, `vite` 8.3.7, `@sveltejs/vite-plugin-svelte` 7.3.1 (peer `vite ^8`, `svelte ^5.46.4`), `vitest` 5.1.0, `@vitest/browser-playwright` 5.1.0, `playwright` 1.65.0, `svelte-check` 4.7.8 (peer `typescript ^5 || ^6`), `typescript` 6.0.3 (la 7.0.2 es *latest* pero incompatible), `@tsconfig/svelte` 5.0.8.
- **Crates:** `tauri` 2.12.5 y `tauri-build` 2.7.4 son las últimas 2.x; crates.io marca como *latest* la 3.0.0-alpha → fijar `"2"`.
- **Trampa de entorno:** en Git Bash, `link.exe` resuelve a `C:\Program Files\Git\usr\bin\link.exe` (no al enlazador MSVC). **Toda compilación Rust y las puertas se ejecutan desde PowerShell.**
- **Referencias:** ARCHITECTURE D5 (sin SvelteKit), D6/§11 (CSP, capabilities mínimas), §4 (estructura), §12 (pruebas), DECISIONES (ventana estándar, barra oscura si el sistema lo está).
- **Riesgos:** CSP estricta bloqueando el HMR de Vite en desarrollo; plantilla de `tauri init` con extras; barra de título que no siga el tema oscuro del sistema.

## 4. Restricciones innegociables

- Nunca `pnpm build`, `pnpm tauri build` ni instaladores (AGENTS.md). `pnpm tauri dev` sí (compilación de depuración).
- Archivos UTF-8 sin BOM, escritos con herramientas de archivo, nunca con redirección de shell.
- Compilación Rust y puertas siempre desde PowerShell, nunca desde Git Bash (ARCHITECTURE §4, Entorno).
- Barra de título oscura: se comprueba en P3; si no sigue al sistema, se aparca en §9 y en `DIFERIDOS.md` para la fase 2, no bloquea esta fase.
- Sin red en la app (D6): CSP de §11; ningún permiso más allá de los listados.
- Sin código de producto (Doc, columnas, prefs, errores): eso es fase 1+.
- Nada de archivos de demo (contadores, logos, `greet`).

## 5. Checklist previo a los pasos

- [x] Estructura del proyecto revisada
- [x] Archivos afectados leídos (no hay código previo)
- [x] Patrones existentes: ARCHITECTURE §4 define la estructura destino
- [x] Decisiones clave consultadas y fijadas (F1–F7)
- [x] Cobertura de pruebas: inexistente; se crea en P4
- [x] Versiones verificadas (§3)
- [x] Consumidores: ninguno

## 6. Pasos secuenciales

### Paso 1 — Línea base: git y entorno
**Archivos:** `.gitignore`, `.gitattributes`
- Comprobar desde PowerShell: `rustc -V`, `pnpm -v`, que `rustup show active-toolchain` termina en `-msvc` y que `& "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath` devuelve al menos una ruta (si no, parar: falta "Desktop development with C++"). No usar `where link`: el enlazador MSVC no está en el PATH fuera de la consola de desarrollador y Rust lo localiza por su cuenta.
- `git init -b main`.
- `.gitignore`: `node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/schemas/`, `Tofu.dmg`, `rea-tofu-evidence.*`, `.vitest-attachments/`, `*.log`.
- `.gitattributes`: `* text=auto eol=lf`.
- Commit `docs: línea base de documentación`. Los commits de este plan terminan con la línea de coautoría del agente que implementa (con Claude Code: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`).

**Gate:** `git status` limpio; `git log --oneline` muestra 1 commit; `git check-ignore Tofu.dmg rea-tofu-evidence.md` devuelve ambos.

### Paso 2 — Frontend mínimo (Svelte 5 + Vite + TS)
**Archivos:** `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `index.html`, `src/main.ts`, `src/App.svelte`, `src/vite-env.d.ts`
- `package.json`: `"name": "miso"`, `"private": true`, `"type": "module"`, `"packageManager": "pnpm@10.33.2"`. Dependencias de desarrollo instaladas con `pnpm add -D` (versiones de §3, `typescript@~6.0`): `svelte`, `vite`, `@sveltejs/vite-plugin-svelte`, `typescript`, `svelte-check`, `@tsconfig/svelte`.
- `vite.config.ts`: plugin svelte; `clearScreen: false`; `server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } }`; `build.target: 'chrome128'` (§10).
- `svelte.config.js`: `vitePreprocess()`.
- `tsconfig.json`: extiende `@tsconfig/svelte`; `strict: true`, `noEmit`, `include: ["src", "tests"]`.
- `index.html`: `lang="es"`, `<meta charset="utf-8">`, `<div id="app">`, `<script type="module" src="/src/main.ts"></script>`.
- `src/main.ts`: `mount(App, { target: document.getElementById('app')! })`.
- `src/App.svelte`: `<main></main>` sin texto; estilo mínimo `:global(html), :global(body), main { margin: 0; height: 100%; background: #fff; }` (sin `:global`, Svelte avisa de selectores sin uso y `--fail-on-warnings` hace fallar `check`).
- Scripts: `"dev": "vite"`, `"build": "vite build"` (lo exige `beforeBuildCommand`; se define, **nunca se ejecuta**), `"check": "svelte-check --tsconfig ./tsconfig.json --fail-on-warnings"`.

**Gate:** `pnpm check` termina con 0 errores y 0 avisos; con `pnpm dev` en marcha, `Invoke-WebRequest http://localhost:1420` devuelve 200 y contiene `id="app"`.

### Paso 3 — `src-tauri/` y ventana vacía
**Archivos:** `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/reader.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/icons/*`, `package.json`
- `pnpm add -D @tauri-apps/cli@^2`; script `"tauri": "tauri"`.
- `pnpm tauri init --ci --app-name Miso --window-title Miso --frontend-dist ../dist --dev-url http://localhost:1420 --before-dev-command "pnpm dev" --before-build-command "pnpm build"` y después ajustar:
  - **Cargo.toml:** `name = "miso"`, `[lib] name = "miso_lib"`, `crate-type = ["staticlib", "cdylib", "rlib"]`, `tauri = { version = "2", features = [] }`, `tauri-build = { version = "2" }`; quitar `tauri-plugin-log`, `log` y cualquier otro plugin; sin `serde` todavía.
  - **lib.rs:** `pub fn run() { tauri::Builder::default().run(tauri::generate_context!()).expect("error al iniciar Miso"); }` — sin `setup` ni plugins.
  - **main.rs:** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` + `miso_lib::run()`.
  - **tauri.conf.json:** `productName: "Miso"`, `identifier: "com.provisional.miso"` (F5); una ventana `{ label: "reader-0", title: "Miso", width: 1000, height: 700, minWidth: 480, minHeight: 360 }`, sin `decorations: false` ni `transparent` (DECISIONES); `app.security.csp: "default-src 'self'; connect-src ipc: http://ipc.localhost"`. Si en desarrollo la CSP bloquea los `<style>` o el HMR que inyecta Vite, relajar **sólo** `app.security.devCsp` (nunca `csp`) con lo mínimo: `csp` + `style-src 'self' 'unsafe-inline'` y `ws://localhost:1420` en `connect-src`. Sin `fileAssociations` (fase 7).
  - **Capabilities:** borrar `default.json`; crear `reader.json` con `identifier: "reader"`, `windows: ["reader-*"]`, `permissions: ["core:event:default", "core:window:default"]` (§11). `prefs.json` se crea en fase 2, con la ventana de preferencias.
  - Mantener los iconos por defecto de `tauri init` (los propios llegan en fase 7).

**Gate (desde PowerShell):** `pnpm tauri dev` compila y abre la ventana; `Get-Process miso | Select MainWindowTitle` devuelve `Miso`; la ventana es blanca y sin contenido; en la consola del WebView (F12), `document.querySelector('#app > main')` no es `null` (Svelte montó) y `fetch('https://example.com')` falla con un error de CSP (`connect-src`), lo que prueba que la CSP está activa; no hay otras violaciones de CSP; con Windows en modo oscuro, la barra de título es oscura. Si esto último no ocurre, **no bloquea la fase**: se aparca en §9 para resolver en fase 2 con `DWMWA_USE_IMMERSIVE_DARK_MODE`. Cerrar la ventana termina el proceso.

### Paso 4 — Puertas locales
**Archivos:** `vitest.config.ts`, `tests/layout/multicol.test.ts`, `package.json`
- `pnpm add -D vitest @vitest/browser-playwright playwright` (sin `playwright install`: se usa Edge, F2).
- `vitest.config.ts`, exactamente con esta forma (`playwright` se importa de `@vitest/browser-playwright`, no del paquete `playwright`; cada proyecto anida su `test` con `name`):

```ts
import { defineConfig } from 'vitest/config'
import { playwright } from '@vitest/browser-playwright'

export default defineConfig({
  test: {
    projects: [
      { test: { name: 'unit', environment: 'node', include: ['src/**/*.test.ts'] } },
      { test: { name: 'layout', include: ['tests/layout/**/*.test.ts'], browser: { enabled: true, headless: true, provider: playwright({ launchOptions: { channel: 'msedge' } }), instances: [{ browser: 'chromium' }] } } },
    ],
  },
})
```
- `tests/layout/multicol.test.ts` — prueba la premisa de D1 en el motor real: un contenedor de `height: 200px; width: 400px; column-width: 100px; column-fill: auto; overflow-x: auto` con texto largo cumple `scrollWidth > clientWidth`, y dos líneas de columnas consecutivas (`Range.getClientRects()`) tienen distinta `left`.
- Scripts:
  - `"test": "vitest run --project unit --passWithNoTests"` (los primeros tests unitarios llegan en fase 1; `--project` exige `name` en cada proyecto)
  - `"test:layout": "vitest run --project layout"`
  - `"lint:rust": "cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings"`
  - `"test:rust": "cargo test --manifest-path src-tauri/Cargo.toml"`
  - `"verify": "pnpm check && pnpm test && pnpm test:layout && pnpm lint:rust && pnpm test:rust"`

**Gate (desde PowerShell):** `pnpm verify` sale con código 0 y `test:layout` informa 1 test superado en Edge. Prueba de que las puertas detectan fallos: un error de tipos temporal en `src/main.ts` hace fallar `pnpm check`, y una variable sin usar en `lib.rs` hace fallar `pnpm lint:rust`. Ambos cambios se revierten.

### Paso 5 — Cierre documental y commit
**Archivos:** `ARCHITECTURE.md`, `REGISTRO.md`, `DIFERIDOS.md`, este plan
- ARCHITECTURE §4: añadir `vitest.config.ts`, `.gitignore`; anotar que `capabilities/prefs.json` llega en fase 2. §5: anotar que se mantiene el *(verificar)* `specta`/`ts-rs` para fase 1.
- `REGISTRO.md`: asiento de la fase 0.
- `DIFERIDOS.md`: los hallazgos aparcados que queden abiertos (p. ej. barra oscura), con fecha y hora.
- Renombrar a `_PLAN_FASE_0_completado.md` y moverlo a `.historial/`.
- Commit `feat: fase 0 — scaffold Tauri 2 + Svelte 5`.

**Gate:** `pnpm verify` en verde sobre el commit final; `git status` limpio; el plan ya no está en la raíz.

## 7. Grafo de dependencias

```
P1 ─→ P2 ─→ P3 ─→ P4 ─→ P5
```
P4 depende de P3 por `lint:rust` y `test:rust`.

## 8. Tabla de seguimiento

| Paso | Estado | Verificación | Notas |
|---|---|---|---|
| P1 — Línea base git y entorno | ✅ completado | 2026-11-10: toolchain msvc, VC Tools 2019/2022; status limpio; commit `0046308` | incluye `.historial/` y el plan |
| P2 — Frontend mínimo | ✅ completado | 2026-11-10: `pnpm check` 0/0; dev 200 en `/` y `/src/main.ts`; puerto 1420 liberado | desviaciones: `.npmrc` `node-linker=hoisted` (E: es exFAT, sin symlinks); `tsconfig` con `allowJs`/`checkJs` (como la plantilla oficial svelte-ts) |
| P3 — `src-tauri/` y ventana vacía | ✅ completado | 2026-11-10: `miso` con título "Miso"; `#app > main` montado (CDP); DWM dark mode = 1 con sistema oscuro; cierre limpio, puerto 1420 libre; `pnpm check` 0/0 | desviación: la CSP no se aplica en `tauri dev` (carga directa de `devUrl`, `eval` permitido, sin meta CSP); no hace falta `devCsp`. Verificación de CSP aparcada a fase 7 |
| P4 — Puertas locales | ✅ completado | 2026-11-11: `pnpm verify` código 0 (check 0/0, unit sin tests, layout 1/1 en EdgeCore 157, clippy `-D warnings`, cargo test); negativas: check 1, clippy 101, layout 1 | desviaciones: Edge no está en ruta estándar (sólo EdgeCore); el implementador creó una junction en `%LOCALAPPDATA%` → **rechazada y borrada**; `vitest.config.ts` resuelve EdgeCore o `channel: 'msedge'`. `.gitignore` + `.vitest/` |
| P5 — Cierre documental y commit | ✅ completado | 2026-11-11: ARCHITECTURE §4/§12, DIFERIDOS, REGISTRO; `pnpm verify` verde | |

Estados: ⬜ pendiente · 🔵 en progreso · ✅ completado · ⛔ bloqueado (indicar causa).

## 9. Fuera de alcance

**Exclusiones deliberadas**
- `AppError`, `report()`, plugins dialog y log, panic hook (fase 1).
- Generación de tipos `specta`/`ts-rs` y `src/lib/ipc/*` (fase 1).
- Ventana de preferencias y `capabilities/prefs.json` (fase 2).
- Single-instance, asociaciones de archivo, iconos propios, `pnpm build` e instalador (fases 3 y 7).
- CI remota.

**Hallazgos aparcados**
- 2026-11-10 — El repositorio vive en un volumen **exFAT** (E:): pnpm necesita `node-linker=hoisted` y los tests de `file_id` (fase 3) deben crear sus archivos en un temporal NTFS (exFAT no da `FileIdInfo` estable).
- 2026-11-10 — En `tauri dev` la CSP no se aplica (la ventana carga `http://localhost:1420` directamente). Comprobar en fase 7, con la app compilada, que `eval` y `fetch` externos quedan bloqueados y que no hay violaciones con los estilos de Svelte.
- 2026-11-10 — Barra de título oscura: verificada (DWM = 1 con el sistema en oscuro). No requiere trabajo en fase 2.
