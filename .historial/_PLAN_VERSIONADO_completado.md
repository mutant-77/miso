# Plan — Versionado de Miso desde 1.0.0

Fecha: 2026-11-15. Estado: implementado (2026-11-15). Release pendiente del build del titular (DIFERIDOS).

## Objetivo

Dar a Miso una versión única, visible y etiquetada en git, empezando por **1.0.0**, con reglas fijas para subirla.

## Situación actual

- `0.1.0` duplicada en `src-tauri/tauri.conf.json` y `src-tauri/Cargo.toml` (diferido del 2026-11-15 07:02).
- `package.json` no tiene versión (`private: true`); no hace falta.
- Ya se usa en el log de arranque (`lib.rs`: `Miso {} iniciado` con `package_info().version`).
- La interfaz no la muestra. No hay etiquetas git. Remoto: `origin` en GitHub.

## Decisiones propuestas

| # | Decisión | Motivo |
|---|---|---|
| V1 | **SemVer `MAYOR.MENOR.PARCHE`**, primera versión `1.0.0`. | Cargo exige tres cifras; «1.0» es `1.0.0`. |
| V2 | **Fuente única: `Cargo.toml`.** Se quita `version` de `tauri.conf.json`. | Tauri 2 toma la de `Cargo.toml` si falta en la config (documentado en su esquema). Cierra el diferido sin scripts de sincronía. |
| V3 | **Se muestra en Preferencias → Acerca de** como «Miso 1.0.0», con `getVersion()` de `@tauri-apps/api/app`. | Un permiso (`core:app:allow-version`) y una línea de Svelte; sin comando Rust nuevo. |
| V4 | **Etiqueta git anotada `vX.Y.Z`** sobre el commit de la versión, publicada en `origin`. | Marca exacta del código de cada versión. |
| V5 | **Sin `CHANGELOG.md`.** Las notas de versión se redactan a partir de `REGISTRO.md` en la etiqueta (y en la Release de GitHub si se decide). | Evita un segundo registro paralelo. |

### Cuándo se sube cada cifra

- **MAYOR:** se rompe algo que el usuario ya tenía: prefs o posiciones que dejan de leerse sin migración, se retira un formato o una versión de Windows.
- **MENOR:** función nueva compatible: formato, idioma, preferencia, atajo.
- **PARCHE:** sólo correcciones, sin cambio de comportamiento previsto.

## Pasos

1. **Fuente única.** `src-tauri/Cargo.toml`: `version = "1.0.0"`. `src-tauri/tauri.conf.json`: borrar la línea `"version"`. `Cargo.lock` se actualiza solo al compilar.
2. **Permiso.** `src-tauri/capabilities/reader.json`: añadir `"core:app:allow-version"`.
3. **Interfaz.** `src/components/Preferences.svelte`, sección «Acerca de»: línea «Miso {versión}» encima del botón Licencias. Sin clave i18n nueva: «Miso» es nombre propio y la versión es un número.
4. **Documentación.**
   - `DECISIONES.md`: una línea «Versión: SemVer desde 1.0.0, fuente única `Cargo.toml`, etiqueta `vX.Y.Z`».
   - `ARCHITECTURE.md` §4: anotar en `Cargo.toml` y `tauri.conf.json` dónde vive la versión; añadir el procedimiento de versión (abajo) en una subsección breve.
   - `DIFERIDOS.md`: quitar «La versión vive duplicada…».
   - `REGISTRO.md`: asiento de la implementación.
5. **Commit** «Versión 1.0.0».
6. **Etiqueta** `git tag -a v1.0.0 -m "Miso 1.0.0"` y `git push origin v1.0.0`. Requiere confirmación del titular (es publicar).
7. **Plan** renombrado a `_PLAN_VERSIONADO_completado.md` y movido a `.historial/`.

## Procedimiento para versiones futuras

1. Decidir la cifra según las reglas.
2. Cambiar `version` en `src-tauri/Cargo.toml`.
3. `pnpm verify` en verde.
4. Asiento en `REGISTRO.md` y commit «Versión X.Y.Z».
5. `git tag -a vX.Y.Z -m "Miso X.Y.Z"` + push de la etiqueta.
6. El titular compila el ejecutable a mano.

## Verificación

- `pnpm verify` en verde.
- `pnpm tauri dev`: el log dice «Miso 1.0.0 iniciado» y Acerca de muestra «Miso 1.0.0» (comprobación visual del titular).
- `git grep -n "0.1.0" -- src-tauri/Cargo.toml src-tauri/tauri.conf.json` sin resultados.
- Tras el build manual del titular: Propiedades → Detalles del `.exe` muestran versión 1.0.0 (tauri-build la toma de la misma fuente).

## Respuestas del titular (2026-11-15)

1. **Formato visible:** «1.0.0».
2. **Release en GitHub:** sí, `v1.0.0` con `miso.exe` adjunto y notas desde `REGISTRO.md`, tras el build manual del titular.
3. **Nombre del ejecutable:** se mantiene `miso.exe`.

## Fuera de alcance

Auto-actualización (excluida por ARCHITECTURE §1), instalador (DIFERIDOS), automatización de releases en CI.
